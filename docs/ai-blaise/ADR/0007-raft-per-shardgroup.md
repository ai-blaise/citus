# ADR 0007: One Raft Group per Shard Group

## Status

Accepted (2026-05-21)

## Context

The overlay needs a consensus layer for shard-level decisions:
placement, leaseholder identity, split / merge, drain, and rebalance.
Citus upstream lacks this primitive — failover relies on external
election services or operator-driven promotions, and shard placement
is metadata in `pg_dist_placement` without strong agreement across
replicas. The choice is between a single global Raft group covering
all topology decisions, one Raft group per shard group, or no Raft at
all (leaning entirely on CNPG plus advisory locks).

## Decision

Run one Raft group per shard group, implemented in `sidecar/raft`
using `raft-rs` (Apache 2.0, from the TiKV project). The shard group
is the consistency boundary for placement and lease decisions: each
group's Raft peers are the replicas of the shards in that group. The
group owns placement (which workers host which shard replicas), the
leaseholder identity (which replica is primary for writes), and the
split/merge state machine. Leader election runs in milliseconds. The
Raft sidecars exchange HLC timestamps (`sidecar/hlc`) on every Raft
RPC; the HLC stream feeds closed-timestamp marks for follower reads.
Raft tells CNPG which pod should be primary for the shard group via
the Kubernetes API; CNPG performs the streaming-replica promotion
(ADR 0006).

## Alternatives considered

- Single global Raft group covering all topology state. Rejected —
  every shard split, every placement change, and every lease renewal
  funnels through one leader. Throughput ceiling is one quorum
  round-trip per decision, and a single leader-flap stalls the whole
  cluster. CockroachDB learned this lesson and moved to range-level
  Raft for the same reason.
- One Raft group per shard. Rejected — too granular. A cluster with
  10k shards and replication factor 3 would have 10k Raft groups and
  30k peers; heartbeat traffic dominates the link. Shard groups
  collapse the count by an order of magnitude while keeping the
  decision boundary aligned with colocation.
- No Raft, rely on CNPG plus advisory locks. Rejected — CNPG's
  primary-election covers the Postgres-lifecycle question (which pod
  is up) but does not order shard-level decisions across replicas.
  Advisory locks do not survive network partitions cleanly.
- Etcd as the consensus store. Rejected — adds an external dependency
  and a separate operational surface. `raft-rs` runs in-process with
  the sidecar fleet and uses the same observability stack.

## Consequences

- Positive: shard-level decisions are linearizable within a shard
  group with millisecond latency. Throughput scales with the number
  of groups, not with a single global leader.
- Positive: the consistency boundary matches colocation — co-located
  shards already share a placement, so they should share a
  decision quorum.
- Positive: integration with CNPG (ADR 0006) is clean — Raft picks
  the leaseholder, CNPG executes the failover.
- Positive: HLC exchange (`sidecar/hlc`) on Raft RPCs provides a
  closed-timestamp source for `AS OF SYSTEM TIME` follower reads
  without extra round-trips.
- Negative: cross-shard-group transactions still need a coordinator
  (see `sidecar/txn_status` and `companion/txn_coord`). Raft alone
  does not give us cross-group atomicity.
- Negative: group membership reconfiguration (shard split, replica
  addition) is a non-trivial Raft joint-consensus operation. The
  state machine in `sidecar/raft` must handle it explicitly.
- Risks: a misconfigured replication factor leaves a Raft group
  without a quorum. Mitigation: the `CitusCluster` validating webhook
  rejects configurations where `replicationFactor < 3` for
  survivability tiers above `ZONE_FAILURE`.

## References

- Plan §6.3.5 (`sidecar/raft`)
- Plan §4.4 (HA without external election service)
- Plan §4.3 (distributed transactions)
- `tikv/raft-rs` — Apache 2.0
- TiKV multi-Raft design notes
- CockroachDB range-level Raft (Stonebraker et al. analyses)
- ADR 0006 — CNPG substrate
