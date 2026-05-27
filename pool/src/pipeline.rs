// FEATURE: T7

//! Extended-query protocol pipelining.
//!
//! PR30 shipped basic simple-query pipelining (`Q` frames). This module
//! extends the pool to also pipeline the extended-query protocol:
//! `Parse / Bind / Describe / Execute` are decoded from the client byte
//! stream, buffered until `Sync` (or the configured `max_in_flight`), and
//! flushed back to the wire as a single batch so latency is dominated by
//! a single RTT amortized across the batch.
//!
//! Framing is delegated to `ai_blaise_citus_pool_wire`; this module owns
//! the buffer-flush policy and the `Sync`-failure semantics.

use crate::{PoolRuntimeError, ProtocolPipelinePolicy};
use ai_blaise_citus_pool_wire::{
    self as wire, BindFrame, DescribeFrame, ExecuteFrame, FlushFrame, ParseFrame, SyncFrame,
};
use std::error::Error;
use std::fmt;

pub use ai_blaise_citus_pool_wire::DescribeTarget;

/// One pgwire extended-query frame the pool will buffer + forward.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExtendedFrame {
    Parse(ParseFrame),
    Bind(BindFrame),
    Describe(DescribeFrame),
    Execute(ExecuteFrame),
    Sync(SyncFrame),
    Flush(FlushFrame),
}

impl ExtendedFrame {
    /// The wire tag this frame would serialize to, useful for log output.
    pub fn wire_tag(&self) -> char {
        match self {
            Self::Parse(_) => 'P',
            Self::Bind(_) => 'B',
            Self::Describe(_) => 'D',
            Self::Execute(_) => 'E',
            Self::Sync(_) => 'S',
            Self::Flush(_) => 'H',
        }
    }

    /// Encode this frame onto a wire buffer in PostgreSQL v3 framing.
    pub fn encode(&self, buf: &mut wire::PgWriteBuf) {
        match self {
            Self::Parse(frame) => frame.encode(buf),
            Self::Bind(frame) => frame.encode(buf),
            Self::Describe(frame) => frame.encode(buf),
            Self::Execute(frame) => frame.encode(buf),
            Self::Sync(frame) => frame.encode(buf),
            Self::Flush(frame) => frame.encode(buf),
        }
    }

    /// Decode a single extended-query frame given the tag and body bytes.
    /// Returns `Ok(None)` when the tag is not one of the extended-query
    /// tags this buffer understands, so callers can hand non-extended
    /// frames (e.g. `Q`, `X`) through the byte-transparent path.
    pub fn decode(tag: u8, body: &[u8]) -> Result<Option<Self>, PipelineError> {
        let frame = match tag {
            b'P' => Self::Parse(ParseFrame::decode(body).map_err(PipelineError::Wire)?),
            b'B' => Self::Bind(BindFrame::decode(body).map_err(PipelineError::Wire)?),
            b'D' => Self::Describe(DescribeFrame::decode(body).map_err(PipelineError::Wire)?),
            b'E' => Self::Execute(ExecuteFrame::decode(body).map_err(PipelineError::Wire)?),
            b'S' => Self::Sync(SyncFrame::decode(body).map_err(PipelineError::Wire)?),
            b'H' => Self::Flush(FlushFrame::decode(body).map_err(PipelineError::Wire)?),
            _ => return Ok(None),
        };
        Ok(Some(frame))
    }

    /// Convenience constructor used by tests that did not previously hold
    /// real wire bytes. Mirrors the legacy `ExtendedFrame::Parse { ... }`
    /// shape so the buffer-policy tests stay readable.
    pub fn new_parse(statement_name: &str, query: &str) -> Self {
        Self::Parse(ParseFrame {
            statement_name: statement_name.to_string(),
            query: query.to_string(),
            parameter_oids: Vec::new(),
        })
    }

    pub fn new_bind(portal_name: &str, statement_name: &str) -> Self {
        Self::Bind(BindFrame {
            portal_name: portal_name.to_string(),
            statement_name: statement_name.to_string(),
            parameter_format_codes: Vec::new(),
            parameters: Vec::new(),
            result_format_codes: Vec::new(),
        })
    }

    pub fn new_describe(target: DescribeTarget, name: &str) -> Self {
        Self::Describe(DescribeFrame {
            target,
            name: name.to_string(),
        })
    }

    pub fn new_execute(portal_name: &str, max_rows: i32) -> Self {
        Self::Execute(ExecuteFrame {
            portal_name: portal_name.to_string(),
            max_rows,
        })
    }

    pub fn new_sync() -> Self {
        Self::Sync(SyncFrame)
    }

    pub fn new_flush() -> Self {
        Self::Flush(FlushFrame)
    }
}

/// Pipeline buffer that respects `ProtocolPipelinePolicy::max_in_flight`.
/// Holds frames between client send and backend flush; `Sync` always flushes
/// the buffer.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExtendedPipelineBuffer {
    max_in_flight: u32,
    transaction_pipelining: bool,
    in_flight: u32,
    buffer: Vec<ExtendedFrame>,
    flushed_batches: u64,
    parse_count: u64,
    bind_count: u64,
    describe_count: u64,
    execute_count: u64,
    sync_count: u64,
    flush_count: u64,
}

impl ExtendedPipelineBuffer {
    pub fn new(policy: &ProtocolPipelinePolicy) -> Result<Self, PipelineError> {
        policy.validate().map_err(PipelineError::Runtime)?;
        Ok(Self {
            max_in_flight: policy.max_in_flight,
            transaction_pipelining: policy.transaction_pipelining,
            in_flight: 0,
            buffer: Vec::new(),
            flushed_batches: 0,
            parse_count: 0,
            bind_count: 0,
            describe_count: 0,
            execute_count: 0,
            sync_count: 0,
            flush_count: 0,
        })
    }

    pub fn transaction_pipelining(&self) -> bool {
        self.transaction_pipelining
    }

    pub fn in_flight(&self) -> u32 {
        self.in_flight
    }

    pub fn flushed_batches(&self) -> u64 {
        self.flushed_batches
    }

    pub fn parse_count(&self) -> u64 {
        self.parse_count
    }

    pub fn bind_count(&self) -> u64 {
        self.bind_count
    }

    pub fn describe_count(&self) -> u64 {
        self.describe_count
    }

    pub fn execute_count(&self) -> u64 {
        self.execute_count
    }

    pub fn sync_count(&self) -> u64 {
        self.sync_count
    }

    pub fn flush_count(&self) -> u64 {
        self.flush_count
    }

    /// Append a frame. If the frame is `Sync` (or the buffer is at the policy
    /// limit) the buffer is returned for the proxy to flush to the backend.
    pub fn append(
        &mut self,
        frame: ExtendedFrame,
    ) -> Result<Option<Vec<ExtendedFrame>>, PipelineError> {
        self.account(&frame);
        if matches!(frame, ExtendedFrame::Sync(_) | ExtendedFrame::Flush(_)) {
            self.buffer.push(frame);
            return Ok(Some(self.flush()));
        }

        if self.in_flight >= self.max_in_flight {
            return Err(PipelineError::Overflow {
                max_in_flight: self.max_in_flight,
            });
        }
        self.in_flight += 1;
        self.buffer.push(frame);

        if self.in_flight >= self.max_in_flight {
            return Ok(Some(self.flush()));
        }
        Ok(None)
    }

    /// Decode the next wire frame out of `bytes` and append it to the
    /// buffer. Returns the buffer if the frame flushed it, and the number
    /// of bytes consumed off the front of `bytes`.
    pub fn append_wire_frame(
        &mut self,
        bytes: &[u8],
    ) -> Result<(Option<Vec<ExtendedFrame>>, usize), PipelineError> {
        let (header, body, consumed) =
            wire::envelope::read_frame(bytes).map_err(PipelineError::Wire)?;
        match ExtendedFrame::decode(header.tag, body)? {
            Some(frame) => {
                let flushed = self.append(frame)?;
                Ok((flushed, consumed))
            }
            None => Err(PipelineError::NonExtendedTag { tag: header.tag }),
        }
    }

    /// Serialize the buffered frames back onto the wire in send order.
    pub fn encode_batch(frames: &[ExtendedFrame]) -> Vec<u8> {
        let mut buf = wire::PgWriteBuf::new();
        for frame in frames {
            frame.encode(&mut buf);
        }
        buf.into_inner()
    }

    fn account(&mut self, frame: &ExtendedFrame) {
        match frame {
            ExtendedFrame::Parse(_) => self.parse_count += 1,
            ExtendedFrame::Bind(_) => self.bind_count += 1,
            ExtendedFrame::Describe(_) => self.describe_count += 1,
            ExtendedFrame::Execute(_) => self.execute_count += 1,
            ExtendedFrame::Sync(_) => self.sync_count += 1,
            ExtendedFrame::Flush(_) => self.flush_count += 1,
        }
    }

    fn flush(&mut self) -> Vec<ExtendedFrame> {
        self.in_flight = 0;
        self.flushed_batches += 1;
        std::mem::take(&mut self.buffer)
    }

    pub fn pending(&self) -> &[ExtendedFrame] {
        &self.buffer
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PipelineError {
    Overflow { max_in_flight: u32 },
    Runtime(PoolRuntimeError),
    Wire(wire::WireError),
    NonExtendedTag { tag: u8 },
}

impl fmt::Display for PipelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow { max_in_flight } => write!(
                formatter,
                "pipeline buffer exceeded max_in_flight={max_in_flight}"
            ),
            Self::Runtime(error) => write!(formatter, "{error}"),
            Self::Wire(error) => write!(formatter, "{error}"),
            Self::NonExtendedTag { tag } => write!(
                formatter,
                "tag 0x{tag:02x} is not an extended-query frame"
            ),
        }
    }
}

impl Error for PipelineError {}

impl From<PoolRuntimeError> for PipelineError {
    fn from(error: PoolRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<wire::WireError> for PipelineError {
    fn from(error: wire::WireError) -> Self {
        Self::Wire(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(max: u32) -> ProtocolPipelinePolicy {
        ProtocolPipelinePolicy {
            max_in_flight: max,
            transaction_pipelining: true,
        }
    }

    #[test]
    fn sync_flushes_buffer() {
        let mut buffer = ExtendedPipelineBuffer::new(&policy(8)).expect("buffer");
        let _ = buffer
            .append(ExtendedFrame::new_parse("s1", "SELECT 1"))
            .expect("parse");
        let _ = buffer
            .append(ExtendedFrame::new_bind("p1", "s1"))
            .expect("bind");
        let _ = buffer
            .append(ExtendedFrame::new_execute("p1", 0))
            .expect("execute");

        let flush = buffer
            .append(ExtendedFrame::new_sync())
            .expect("sync")
            .expect("flush");

        assert_eq!(flush.len(), 4);
        assert_eq!(flush[0].wire_tag(), 'P');
        assert_eq!(flush[3].wire_tag(), 'S');
        assert_eq!(buffer.in_flight(), 0);
        assert_eq!(buffer.flushed_batches(), 1);
        assert_eq!(buffer.parse_count(), 1);
        assert_eq!(buffer.execute_count(), 1);
        assert_eq!(buffer.sync_count(), 1);
    }

    #[test]
    fn reaching_max_in_flight_auto_flushes() {
        let mut buffer = ExtendedPipelineBuffer::new(&policy(2)).expect("buffer");
        assert!(buffer
            .append(ExtendedFrame::new_parse("s1", "SELECT 1"))
            .expect("parse")
            .is_none());
        let flush = buffer
            .append(ExtendedFrame::new_bind("p1", "s1"))
            .expect("bind");
        let frames = flush.expect("autoflush");
        assert_eq!(frames.len(), 2);
        assert_eq!(buffer.in_flight(), 0);
    }

    #[test]
    fn flush_returns_for_explicit_flush_frame() {
        let mut buffer = ExtendedPipelineBuffer::new(&policy(4)).expect("buffer");
        let _ = buffer
            .append(ExtendedFrame::new_parse("s1", "SELECT 1"))
            .expect("parse");
        let frames = buffer
            .append(ExtendedFrame::new_flush())
            .expect("flush")
            .expect("frames");
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1].wire_tag(), 'H');
    }

    #[test]
    fn invalid_max_in_flight_rejected() {
        let policy = ProtocolPipelinePolicy {
            max_in_flight: 0,
            transaction_pipelining: true,
        };
        assert!(matches!(
            ExtendedPipelineBuffer::new(&policy),
            Err(PipelineError::Runtime(
                PoolRuntimeError::InvalidPipelineDepth
            ))
        ));
    }

    #[test]
    fn append_wire_frame_decodes_real_bytes() {
        let mut buf = wire::PgWriteBuf::new();
        ExtendedFrame::new_parse("s1", "SELECT 1").encode(&mut buf);
        ExtendedFrame::new_bind("p1", "s1").encode(&mut buf);
        ExtendedFrame::new_execute("p1", 0).encode(&mut buf);
        ExtendedFrame::new_sync().encode(&mut buf);
        let wire_bytes = buf.into_inner();

        let mut buffer = ExtendedPipelineBuffer::new(&policy(8)).expect("buffer");
        let mut cursor = 0;
        let mut flushed: Option<Vec<ExtendedFrame>> = None;
        while cursor < wire_bytes.len() {
            let (maybe_flush, consumed) = buffer
                .append_wire_frame(&wire_bytes[cursor..])
                .expect("decode");
            cursor += consumed;
            if maybe_flush.is_some() {
                flushed = maybe_flush;
            }
        }
        let frames = flushed.expect("sync flushed");
        assert_eq!(frames.len(), 4);
        let round = ExtendedPipelineBuffer::encode_batch(&frames);
        assert_eq!(round, wire_bytes);
    }

    #[test]
    fn append_wire_frame_rejects_non_extended_tag() {
        // 'Q' (simple query) is not an extended-query frame.
        let mut buf = wire::PgWriteBuf::new();
        wire::QueryFrame {
            query: "SELECT 1;".to_string(),
        }
        .encode(&mut buf);
        let bytes = buf.into_inner();
        let mut buffer = ExtendedPipelineBuffer::new(&policy(8)).expect("buffer");
        assert!(matches!(
            buffer.append_wire_frame(&bytes),
            Err(PipelineError::NonExtendedTag { tag: b'Q' })
        ));
    }
}
