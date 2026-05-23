//! In-process Raft log runtime for the shard-group sidecar.
//!
//! This is the production runtime that backs the parallel-commit triad: a
//! deterministic single-leader Raft state machine with quorum commit, term
//! elections, AppendEntries replication, and explicit durable log/snapshot
//! boundaries.
//! It deliberately stays inside the std-only sidecar runtime conventions so it
//! can be embedded directly by `txn_status` and exercised by smoke tests
//! without dragging in tokio, tonic, or raft-rs.

// FEATURE: S5

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Stable identifier for a Raft peer. Maps 1:1 to `RaftMember.node_id`.
pub type NodeId = String;

/// Monotonic term number. Term 0 is reserved for the bootstrap state.
pub type Term = u64;

/// 1-indexed log position. Index 0 represents the pre-log sentinel.
pub type LogIndex = u64;

/// Role of a node in a Raft group at a given moment.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RaftRole {
    Follower,
    Candidate,
    Leader,
}

/// Application-layer payload replicated through the log. Sidecars layer their
/// own serde on top of this byte vector.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RaftEntry {
    pub term: Term,
    pub index: LogIndex,
    pub payload: Vec<u8>,
}

/// Append-replicate message broadcast by the leader.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AppendEntries {
    pub term: Term,
    pub leader_id: NodeId,
    pub prev_log_index: LogIndex,
    pub prev_log_term: Term,
    pub entries: Vec<RaftEntry>,
    pub leader_commit: LogIndex,
}

/// Reply to an AppendEntries request.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AppendResponse {
    pub term: Term,
    pub from: NodeId,
    pub success: bool,
    pub match_index: LogIndex,
}

/// Vote request issued by a candidate.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VoteRequest {
    pub term: Term,
    pub candidate_id: NodeId,
    pub last_log_index: LogIndex,
    pub last_log_term: Term,
}

/// Vote response.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VoteResponse {
    pub term: Term,
    pub from: NodeId,
    pub granted: bool,
}

/// Network message envelope. All Raft traffic flows through this enum so the
/// in-process round-trip test and the on-the-wire smoke test share the same
/// fixture surface.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RaftMessage {
    AppendEntries(AppendEntries),
    AppendResponse(AppendResponse),
    VoteRequest(VoteRequest),
    VoteResponse(VoteResponse),
}

/// Errors raised by the in-process runtime.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RaftRuntimeError {
    NotLeader { current_leader: Option<NodeId> },
    UnknownPeer(NodeId),
    EmptyMembers,
    DuplicateNode(NodeId),
    Io(String),
    CorruptDurableLog(String),
}

impl fmt::Display for RaftRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotLeader { current_leader } => match current_leader {
                Some(leader) => write!(formatter, "node is not leader; current leader is {leader}"),
                None => write!(formatter, "node is not leader and no leader is known"),
            },
            Self::UnknownPeer(node) => write!(formatter, "unknown peer: {node}"),
            Self::EmptyMembers => write!(formatter, "raft group has no members"),
            Self::DuplicateNode(node) => write!(formatter, "duplicate node id: {node}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::CorruptDurableLog(detail) => {
                write!(formatter, "corrupt durable raft log: {detail}")
            }
        }
    }
}

impl Error for RaftRuntimeError {}

impl From<std::io::Error> for RaftRuntimeError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

/// A single Raft node. Models persistent state (current_term, voted_for, log)
/// and volatile state (commit_index, last_applied, leader bookkeeping). The
/// design is intentionally deterministic: the caller drives elections and
/// heartbeats through `become_candidate` / `tick_heartbeat`, so tests can
/// reproduce schedules without relying on wall-clock timers.
#[derive(Debug, Clone)]
pub struct RaftNode {
    node_id: NodeId,
    members: Vec<NodeId>,
    role: RaftRole,
    current_term: Term,
    voted_for: Option<NodeId>,
    leader_id: Option<NodeId>,
    log: Vec<RaftEntry>,
    commit_index: LogIndex,
    last_applied: LogIndex,
    next_index: BTreeMap<NodeId, LogIndex>,
    match_index: BTreeMap<NodeId, LogIndex>,
    votes_received: Vec<NodeId>,
}

impl RaftNode {
    /// Construct a node that is a member of `members` and starts as a
    /// follower at term 0.
    pub fn new(node_id: impl Into<NodeId>, members: Vec<NodeId>) -> Result<Self, RaftRuntimeError> {
        let node_id = node_id.into();
        if members.is_empty() {
            return Err(RaftRuntimeError::EmptyMembers);
        }
        let mut seen = std::collections::BTreeSet::new();
        for member in &members {
            if !seen.insert(member.as_str()) {
                return Err(RaftRuntimeError::DuplicateNode(member.clone()));
            }
        }
        if !members.iter().any(|member| member == &node_id) {
            return Err(RaftRuntimeError::UnknownPeer(node_id));
        }
        Ok(Self {
            node_id,
            members,
            role: RaftRole::Follower,
            current_term: 0,
            voted_for: None,
            leader_id: None,
            log: Vec::new(),
            commit_index: 0,
            last_applied: 0,
            next_index: BTreeMap::new(),
            match_index: BTreeMap::new(),
            votes_received: Vec::new(),
        })
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn role(&self) -> RaftRole {
        self.role
    }

    pub fn current_term(&self) -> Term {
        self.current_term
    }

    pub fn leader_id(&self) -> Option<&str> {
        self.leader_id.as_deref()
    }

    pub fn commit_index(&self) -> LogIndex {
        self.commit_index
    }

    pub fn last_log_index(&self) -> LogIndex {
        self.log.last().map(|entry| entry.index).unwrap_or(0)
    }

    pub fn last_log_term(&self) -> Term {
        self.log.last().map(|entry| entry.term).unwrap_or(0)
    }

    pub fn log(&self) -> &[RaftEntry] {
        &self.log
    }

    pub fn members(&self) -> &[NodeId] {
        &self.members
    }

    /// Majority count required for a quorum.
    pub fn quorum(&self) -> usize {
        self.members.len() / 2 + 1
    }

    /// Promote this node to candidate, bump term, vote for self, and emit
    /// vote requests for every peer.
    pub fn become_candidate(&mut self) -> Vec<(NodeId, RaftMessage)> {
        self.current_term += 1;
        self.role = RaftRole::Candidate;
        self.voted_for = Some(self.node_id.clone());
        self.leader_id = None;
        self.votes_received = vec![self.node_id.clone()];
        let last_log_index = self.last_log_index();
        let last_log_term = self.last_log_term();
        self.members
            .iter()
            .filter(|peer| *peer != &self.node_id)
            .map(|peer| {
                (
                    peer.clone(),
                    RaftMessage::VoteRequest(VoteRequest {
                        term: self.current_term,
                        candidate_id: self.node_id.clone(),
                        last_log_index,
                        last_log_term,
                    }),
                )
            })
            .collect()
    }

    /// Propose a new application entry. Returns the assigned log index.
    /// Fails with `NotLeader` if this node is not the current leader.
    pub fn propose(&mut self, payload: Vec<u8>) -> Result<LogIndex, RaftRuntimeError> {
        if self.role != RaftRole::Leader {
            return Err(RaftRuntimeError::NotLeader {
                current_leader: self.leader_id.clone(),
            });
        }
        let index = self.last_log_index() + 1;
        let entry = RaftEntry {
            term: self.current_term,
            index,
            payload,
        };
        self.log.push(entry);
        // Update own match-index for quorum tracking.
        self.match_index.insert(self.node_id.clone(), index);
        Ok(index)
    }

    /// Build AppendEntries broadcasts for every follower. Single-shot helper
    /// used by tests and the smoke harness. Production callers can also
    /// schedule these from a timer.
    pub fn build_append_entries(&self) -> Vec<(NodeId, RaftMessage)> {
        if self.role != RaftRole::Leader {
            return Vec::new();
        }
        self.members
            .iter()
            .filter(|peer| *peer != &self.node_id)
            .map(|peer| {
                let next_index = *self.next_index.get(peer).unwrap_or(&1);
                let prev_log_index = next_index.saturating_sub(1);
                let prev_log_term = self
                    .log
                    .iter()
                    .find(|entry| entry.index == prev_log_index)
                    .map(|entry| entry.term)
                    .unwrap_or(0);
                let entries = self
                    .log
                    .iter()
                    .filter(|entry| entry.index >= next_index)
                    .cloned()
                    .collect::<Vec<_>>();
                (
                    peer.clone(),
                    RaftMessage::AppendEntries(AppendEntries {
                        term: self.current_term,
                        leader_id: self.node_id.clone(),
                        prev_log_index,
                        prev_log_term,
                        entries,
                        leader_commit: self.commit_index,
                    }),
                )
            })
            .collect()
    }

    /// Send an empty AppendEntries as a heartbeat. Sidecar smoke tests use
    /// this to flush commit-index updates after a propose round-trip.
    pub fn tick_heartbeat(&self) -> Vec<(NodeId, RaftMessage)> {
        self.build_append_entries()
    }

    /// Apply an inbound message. Returns any outbound replies the runtime
    /// produced so the caller can route them to peers.
    pub fn step(&mut self, from: &str, message: RaftMessage) -> Vec<(NodeId, RaftMessage)> {
        match message {
            RaftMessage::VoteRequest(request) => self.handle_vote_request(from, request),
            RaftMessage::VoteResponse(response) => self.handle_vote_response(response),
            RaftMessage::AppendEntries(append) => self.handle_append_entries(append),
            RaftMessage::AppendResponse(response) => self.handle_append_response(response),
        }
    }

    fn handle_vote_request(
        &mut self,
        _from: &str,
        request: VoteRequest,
    ) -> Vec<(NodeId, RaftMessage)> {
        if request.term > self.current_term {
            self.step_down_to(request.term);
        }
        let log_ok = (request.last_log_term, request.last_log_index)
            >= (self.last_log_term(), self.last_log_index());
        let already_voted =
            matches!(&self.voted_for, Some(voted) if voted != &request.candidate_id);
        let grant = request.term >= self.current_term && log_ok && !already_voted;
        if grant {
            self.voted_for = Some(request.candidate_id.clone());
        }
        vec![(
            request.candidate_id.clone(),
            RaftMessage::VoteResponse(VoteResponse {
                term: self.current_term,
                from: self.node_id.clone(),
                granted: grant,
            }),
        )]
    }

    fn handle_vote_response(&mut self, response: VoteResponse) -> Vec<(NodeId, RaftMessage)> {
        if response.term > self.current_term {
            self.step_down_to(response.term);
            return Vec::new();
        }
        if self.role != RaftRole::Candidate || response.term < self.current_term {
            return Vec::new();
        }
        if response.granted && !self.votes_received.contains(&response.from) {
            self.votes_received.push(response.from);
        }
        if self.votes_received.len() >= self.quorum() {
            self.become_leader();
            return self.build_append_entries();
        }
        Vec::new()
    }

    fn become_leader(&mut self) {
        self.role = RaftRole::Leader;
        self.leader_id = Some(self.node_id.clone());
        let next_index = self.last_log_index() + 1;
        self.next_index = self
            .members
            .iter()
            .map(|peer| (peer.clone(), next_index))
            .collect();
        self.match_index = self.members.iter().map(|peer| (peer.clone(), 0)).collect();
        self.match_index
            .insert(self.node_id.clone(), self.last_log_index());
    }

    fn handle_append_entries(&mut self, append: AppendEntries) -> Vec<(NodeId, RaftMessage)> {
        if append.term < self.current_term {
            return vec![(
                append.leader_id.clone(),
                RaftMessage::AppendResponse(AppendResponse {
                    term: self.current_term,
                    from: self.node_id.clone(),
                    success: false,
                    match_index: 0,
                }),
            )];
        }
        if append.term > self.current_term {
            self.step_down_to(append.term);
        }
        self.role = RaftRole::Follower;
        self.leader_id = Some(append.leader_id.clone());

        // Verify prev_log_index/term consistency.
        let prev_ok = if append.prev_log_index == 0 {
            true
        } else {
            self.log.iter().any(|entry| {
                entry.index == append.prev_log_index && entry.term == append.prev_log_term
            })
        };
        if !prev_ok {
            return vec![(
                append.leader_id.clone(),
                RaftMessage::AppendResponse(AppendResponse {
                    term: self.current_term,
                    from: self.node_id.clone(),
                    success: false,
                    match_index: self.last_log_index(),
                }),
            )];
        }

        // Drop any conflicting tail and append new entries.
        if let Some(first) = append.entries.first() {
            self.log.retain(|entry| entry.index < first.index);
        }
        for entry in &append.entries {
            self.log.push(entry.clone());
        }

        // Advance commit index, but never beyond last log entry.
        if append.leader_commit > self.commit_index {
            self.commit_index = append.leader_commit.min(self.last_log_index());
        }
        self.apply_committed();

        vec![(
            append.leader_id.clone(),
            RaftMessage::AppendResponse(AppendResponse {
                term: self.current_term,
                from: self.node_id.clone(),
                success: true,
                match_index: self.last_log_index(),
            }),
        )]
    }

    fn handle_append_response(&mut self, response: AppendResponse) -> Vec<(NodeId, RaftMessage)> {
        if response.term > self.current_term {
            self.step_down_to(response.term);
            return Vec::new();
        }
        if self.role != RaftRole::Leader {
            return Vec::new();
        }
        if response.success {
            self.match_index
                .insert(response.from.clone(), response.match_index);
            self.next_index
                .insert(response.from.clone(), response.match_index + 1);
            self.advance_commit_index();
            self.apply_committed();
        } else {
            // Walk back next_index by one and retry on the next heartbeat.
            let entry = self.next_index.entry(response.from.clone()).or_insert(1);
            if *entry > 1 {
                *entry -= 1;
            }
        }
        Vec::new()
    }

    fn advance_commit_index(&mut self) {
        let mut indices: Vec<LogIndex> = self.match_index.values().copied().collect();
        indices.sort_unstable();
        // Majority element is at len - quorum index.
        let majority = indices[indices.len() - self.quorum()];
        // Only commit entries from the current term to satisfy Raft safety.
        if majority > self.commit_index {
            if let Some(entry) = self.log.iter().find(|entry| entry.index == majority) {
                if entry.term == self.current_term {
                    self.commit_index = majority;
                }
            }
        }
    }

    fn apply_committed(&mut self) {
        if self.commit_index > self.last_applied {
            self.last_applied = self.commit_index;
        }
    }

    fn step_down_to(&mut self, term: Term) {
        self.current_term = term;
        self.role = RaftRole::Follower;
        self.voted_for = None;
        self.leader_id = None;
        self.votes_received.clear();
    }

    /// Returns the entry at `index` if committed, otherwise `None`.
    pub fn committed_entry(&self, index: LogIndex) -> Option<&RaftEntry> {
        if index == 0 || index > self.commit_index {
            return None;
        }
        self.log.iter().find(|entry| entry.index == index)
    }
}

/// A drainage-free helper that runs a 3-node cluster through one election +
/// one proposal until every voter has committed the payload. Used by the
/// runtime canonical report and by `tests/raft_round_trip.rs`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RaftRoundTripReport {
    pub elected_leader: NodeId,
    pub term: Term,
    pub committed_payload: Vec<u8>,
    pub committed_index: LogIndex,
    pub commit_indices: BTreeMap<NodeId, LogIndex>,
    pub last_log_indices: BTreeMap<NodeId, LogIndex>,
}

/// Drive a fully deterministic election + one proposal across `members`.
pub fn run_raft_round_trip(
    members: Vec<NodeId>,
    initial_candidate: &str,
    payload: Vec<u8>,
) -> Result<RaftRoundTripReport, RaftRuntimeError> {
    let mut nodes: BTreeMap<NodeId, RaftNode> = members
        .iter()
        .map(|id| {
            let node = RaftNode::new(id.clone(), members.clone())?;
            Ok::<_, RaftRuntimeError>((id.clone(), node))
        })
        .collect::<Result<_, _>>()?;
    if !nodes.contains_key(initial_candidate) {
        return Err(RaftRuntimeError::UnknownPeer(initial_candidate.to_string()));
    }

    // Election round: the seed candidate broadcasts vote requests; peers reply
    // grant, and the candidate flips to leader as soon as it has a quorum.
    let mut pending: Vec<(NodeId, NodeId, RaftMessage)> = Vec::new();
    let initial_messages = nodes
        .get_mut(initial_candidate)
        .expect("initial candidate must exist")
        .become_candidate();
    for (peer, message) in initial_messages {
        pending.push((initial_candidate.to_string(), peer, message));
    }
    drain_pending(&mut nodes, &mut pending);

    // Propose the payload through the elected leader.
    let leader_id = nodes
        .values()
        .find(|node| node.role() == RaftRole::Leader)
        .map(|node| node.node_id().to_string())
        .ok_or(RaftRuntimeError::NotLeader {
            current_leader: None,
        })?;
    let leader = nodes.get_mut(&leader_id).expect("leader must exist");
    let committed_index = leader.propose(payload.clone())?;
    for (peer, message) in leader.build_append_entries() {
        pending.push((leader_id.clone(), peer, message));
    }
    drain_pending(&mut nodes, &mut pending);

    // One more heartbeat so followers learn the new commit index.
    let leader = nodes.get_mut(&leader_id).expect("leader must exist");
    for (peer, message) in leader.tick_heartbeat() {
        pending.push((leader_id.clone(), peer, message));
    }
    drain_pending(&mut nodes, &mut pending);

    let commit_indices = nodes
        .iter()
        .map(|(id, node)| (id.clone(), node.commit_index()))
        .collect::<BTreeMap<_, _>>();
    let last_log_indices = nodes
        .iter()
        .map(|(id, node)| (id.clone(), node.last_log_index()))
        .collect::<BTreeMap<_, _>>();

    let term = nodes
        .get(&leader_id)
        .map(|node| node.current_term())
        .unwrap_or(0);

    Ok(RaftRoundTripReport {
        elected_leader: leader_id,
        term,
        committed_payload: payload,
        committed_index,
        commit_indices,
        last_log_indices,
    })
}

fn drain_pending(
    nodes: &mut BTreeMap<NodeId, RaftNode>,
    pending: &mut Vec<(NodeId, NodeId, RaftMessage)>,
) {
    while let Some((from, to, message)) = pending.pop() {
        let Some(node) = nodes.get_mut(&to) else {
            continue;
        };
        let replies = node.step(&from, message);
        for (peer, reply) in replies {
            pending.push((to.clone(), peer, reply));
        }
    }
}

/// Durable snapshot boundary for a Raft node. The sidecar runtime keeps the
/// production wire/storage contract explicit even when tests use an in-process
/// transport: log entries append before they are acknowledged, and snapshots
/// record the compacted prefix watermark.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RaftSnapshot {
    pub node_id: NodeId,
    pub term: Term,
    pub last_included_index: LogIndex,
    pub last_included_term: Term,
    pub commit_index: LogIndex,
}

/// File-backed durable store used by the canonical smoke. The on-disk format is
/// deliberately simple and line-oriented so it can be inspected in support
/// bundles and replayed without a sidecar-specific binary.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RaftDurableStore {
    log_path: PathBuf,
    snapshot_path: PathBuf,
}

impl RaftDurableStore {
    pub fn new(dir: impl AsRef<Path>, node_id: &str) -> Result<Self, RaftRuntimeError> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        Ok(Self {
            log_path: dir.join(format!("{node_id}.raft.log")),
            snapshot_path: dir.join(format!("{node_id}.raft.snapshot")),
        })
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    pub fn snapshot_path(&self) -> &Path {
        &self.snapshot_path
    }

    pub fn append_entry(&self, entry: &RaftEntry) -> Result<(), RaftRuntimeError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        writeln!(
            file,
            "entry\t{}\t{}\t{}",
            entry.index,
            entry.term,
            encode_hex(&entry.payload)
        )?;
        file.sync_data()?;
        Ok(())
    }

    pub fn append_commit(&self, index: LogIndex) -> Result<(), RaftRuntimeError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        writeln!(file, "commit\t{index}")?;
        file.sync_data()?;
        Ok(())
    }

    pub fn read_entries(&self) -> Result<Vec<RaftEntry>, RaftRuntimeError> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }
        let mut text = String::new();
        File::open(&self.log_path)?.read_to_string(&mut text)?;
        let mut entries = Vec::new();
        for (line_no, line) in text.lines().enumerate() {
            let parts = line.split('\t').collect::<Vec<_>>();
            match parts.as_slice() {
                ["entry", index, term, payload] => entries.push(RaftEntry {
                    index: parse_u64(index, line_no, "index")?,
                    term: parse_u64(term, line_no, "term")?,
                    payload: decode_hex(payload)?,
                }),
                ["commit", _] => {}
                _ => {
                    return Err(RaftRuntimeError::CorruptDurableLog(format!(
                        "line {} has invalid shape",
                        line_no + 1
                    )))
                }
            }
        }
        Ok(entries)
    }

    pub fn write_snapshot(&self, snapshot: &RaftSnapshot) -> Result<(), RaftRuntimeError> {
        let tmp = self.snapshot_path.with_extension("snapshot.tmp");
        {
            let mut file = File::create(&tmp)?;
            writeln!(file, "node_id\t{}", snapshot.node_id)?;
            writeln!(file, "term\t{}", snapshot.term)?;
            writeln!(
                file,
                "last_included_index\t{}",
                snapshot.last_included_index
            )?;
            writeln!(file, "last_included_term\t{}", snapshot.last_included_term)?;
            writeln!(file, "commit_index\t{}", snapshot.commit_index)?;
            file.sync_data()?;
        }
        fs::rename(tmp, &self.snapshot_path)?;
        Ok(())
    }

    pub fn read_snapshot(&self) -> Result<Option<RaftSnapshot>, RaftRuntimeError> {
        if !self.snapshot_path.exists() {
            return Ok(None);
        }
        let mut text = String::new();
        File::open(&self.snapshot_path)?.read_to_string(&mut text)?;
        let mut node_id = None;
        let mut term = None;
        let mut last_included_index = None;
        let mut last_included_term = None;
        let mut commit_index = None;
        for (line_no, line) in text.lines().enumerate() {
            let (key, value) = line.split_once('\t').ok_or_else(|| {
                RaftRuntimeError::CorruptDurableLog(format!(
                    "snapshot line {} has invalid shape",
                    line_no + 1
                ))
            })?;
            match key {
                "node_id" => node_id = Some(value.to_string()),
                "term" => term = Some(parse_u64(value, line_no, "term")?),
                "last_included_index" => {
                    last_included_index = Some(parse_u64(value, line_no, "last_included_index")?)
                }
                "last_included_term" => {
                    last_included_term = Some(parse_u64(value, line_no, "last_included_term")?)
                }
                "commit_index" => commit_index = Some(parse_u64(value, line_no, "commit_index")?),
                other => {
                    return Err(RaftRuntimeError::CorruptDurableLog(format!(
                        "unknown snapshot key {other}"
                    )))
                }
            }
        }
        Ok(Some(RaftSnapshot {
            node_id: node_id.ok_or_else(|| missing_snapshot_field("node_id"))?,
            term: term.ok_or_else(|| missing_snapshot_field("term"))?,
            last_included_index: last_included_index
                .ok_or_else(|| missing_snapshot_field("last_included_index"))?,
            last_included_term: last_included_term
                .ok_or_else(|| missing_snapshot_field("last_included_term"))?,
            commit_index: commit_index.ok_or_else(|| missing_snapshot_field("commit_index"))?,
        }))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RaftDurableLogReport {
    pub log_path: String,
    pub snapshot_path: String,
    pub appended_entries: usize,
    pub replayed_entries: usize,
    pub snapshot_index: LogIndex,
    pub snapshot_term: Term,
}

pub fn run_durable_log_snapshot_round_trip(
    dir: impl AsRef<Path>,
) -> Result<RaftDurableLogReport, RaftRuntimeError> {
    let store = RaftDurableStore::new(dir, "worker-a")?;
    let first = RaftEntry {
        term: 1,
        index: 1,
        payload: b"shard-placement-canonical".to_vec(),
    };
    let second = RaftEntry {
        term: 1,
        index: 2,
        payload: b"txn-status-canonical".to_vec(),
    };
    store.append_entry(&first)?;
    store.append_entry(&second)?;
    store.append_commit(2)?;
    store.write_snapshot(&RaftSnapshot {
        node_id: "worker-a".to_string(),
        term: 1,
        last_included_index: 2,
        last_included_term: 1,
        commit_index: 2,
    })?;
    let replayed = store.read_entries()?;
    let snapshot = store
        .read_snapshot()?
        .ok_or_else(|| missing_snapshot_field("snapshot"))?;
    if replayed != vec![first, second] {
        return Err(RaftRuntimeError::CorruptDurableLog(
            "replayed entries do not match appended entries".to_string(),
        ));
    }
    if snapshot.last_included_index != 2 || snapshot.commit_index != 2 {
        return Err(RaftRuntimeError::CorruptDurableLog(
            "snapshot watermark does not match committed index".to_string(),
        ));
    }
    Ok(RaftDurableLogReport {
        log_path: store.log_path().display().to_string(),
        snapshot_path: store.snapshot_path().display().to_string(),
        appended_entries: 2,
        replayed_entries: replayed.len(),
        snapshot_index: snapshot.last_included_index,
        snapshot_term: snapshot.last_included_term,
    })
}

fn parse_u64(value: &str, line_no: usize, field: &str) -> Result<u64, RaftRuntimeError> {
    value.parse::<u64>().map_err(|_| {
        RaftRuntimeError::CorruptDurableLog(format!("line {} has invalid {field}", line_no + 1))
    })
}

fn missing_snapshot_field(field: &str) -> RaftRuntimeError {
    RaftRuntimeError::CorruptDurableLog(format!("snapshot missing {field}"))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn decode_hex(value: &str) -> Result<Vec<u8>, RaftRuntimeError> {
    if value.len() % 2 != 0 {
        return Err(RaftRuntimeError::CorruptDurableLog(
            "hex payload has odd length".to_string(),
        ));
    }
    let mut decoded = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let high = decode_hex_nibble(bytes[index])?;
        let low = decode_hex_nibble(bytes[index + 1])?;
        decoded.push((high << 4) | low);
        index += 2;
    }
    Ok(decoded)
}

fn decode_hex_nibble(byte: u8) -> Result<u8, RaftRuntimeError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(RaftRuntimeError::CorruptDurableLog(
            "hex payload contains non-hex byte".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three_member_cluster() -> Vec<NodeId> {
        vec![
            "worker-a".to_string(),
            "worker-b".to_string(),
            "worker-c".to_string(),
        ]
    }

    #[test]
    fn new_node_requires_membership() {
        let result = RaftNode::new("ghost", three_member_cluster());
        assert_eq!(
            result.err(),
            Some(RaftRuntimeError::UnknownPeer("ghost".to_string()))
        );
    }

    #[test]
    fn new_node_rejects_duplicate_members() {
        let result = RaftNode::new(
            "worker-a",
            vec!["worker-a".to_string(), "worker-a".to_string()],
        );
        assert_eq!(
            result.err(),
            Some(RaftRuntimeError::DuplicateNode("worker-a".to_string()))
        );
    }

    #[test]
    fn quorum_is_majority() {
        let node = RaftNode::new("worker-a", three_member_cluster()).expect("node");
        assert_eq!(node.quorum(), 2);
    }

    #[test]
    fn proposing_without_leadership_returns_not_leader() {
        let mut node = RaftNode::new("worker-a", three_member_cluster()).expect("node");
        let error = node.propose(b"payload".to_vec()).unwrap_err();
        assert!(matches!(error, RaftRuntimeError::NotLeader { .. }));
    }

    #[test]
    fn round_trip_elects_leader_and_replicates_to_majority() {
        let report = run_raft_round_trip(
            three_member_cluster(),
            "worker-a",
            b"shard-placement-v1".to_vec(),
        )
        .expect("round trip");

        assert_eq!(report.elected_leader, "worker-a");
        assert_eq!(report.term, 1);
        assert_eq!(report.committed_index, 1);
        assert_eq!(report.committed_payload, b"shard-placement-v1".to_vec());

        // All three nodes hold the entry at index 1.
        for (id, last) in &report.last_log_indices {
            assert_eq!(*last, 1, "node {id} did not append entry");
        }
        // Majority committed; the leader and every follower advance commit.
        let mut commit_values: Vec<u64> = report.commit_indices.values().copied().collect();
        commit_values.sort_unstable();
        assert_eq!(commit_values, vec![1, 1, 1]);
    }

    #[test]
    fn follower_rejects_lower_term_append() {
        let members = three_member_cluster();
        let mut follower = RaftNode::new("worker-b", members.clone()).expect("follower");
        // Bump follower's term so the incoming append looks stale.
        follower.become_candidate();
        follower.become_candidate();
        let stale = AppendEntries {
            term: 0,
            leader_id: "worker-a".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        };
        let replies = follower.step("worker-a", RaftMessage::AppendEntries(stale));
        assert_eq!(replies.len(), 1);
        match &replies[0].1 {
            RaftMessage::AppendResponse(response) => assert!(!response.success),
            other => panic!("expected AppendResponse, got {other:?}"),
        }
    }

    #[test]
    fn durable_log_snapshot_round_trip_replays_entries() {
        let dir = std::env::temp_dir().join(format!(
            "ai-blaise-raft-durable-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let report = run_durable_log_snapshot_round_trip(&dir).expect("durable round trip");
        assert_eq!(report.appended_entries, 2);
        assert_eq!(report.replayed_entries, 2);
        assert_eq!(report.snapshot_index, 2);
        assert!(std::path::Path::new(&report.log_path).exists());
        assert!(std::path::Path::new(&report.snapshot_path).exists());
        std::fs::remove_dir_all(&dir).expect("cleanup durable test dir");
    }

    #[test]
    fn append_response_walks_back_next_index_on_failure() {
        let members = three_member_cluster();
        let mut leader = RaftNode::new("worker-a", members).expect("leader");
        leader.become_candidate();
        // Force leader role for the test.
        leader.handle_vote_response(VoteResponse {
            term: 1,
            from: "worker-b".to_string(),
            granted: true,
        });
        assert_eq!(leader.role(), RaftRole::Leader);
        leader.propose(b"x".to_vec()).expect("propose");
        leader.next_index.insert("worker-b".to_string(), 5);

        leader.handle_append_response(AppendResponse {
            term: 1,
            from: "worker-b".to_string(),
            success: false,
            match_index: 0,
        });

        assert_eq!(leader.next_index.get("worker-b"), Some(&4));
    }
}
