// FEATURE: T7

//! Extended-query protocol pipelining.
//!
//! PR30 shipped basic simple-query pipelining (`Q` frames). This module
//! extends the pool to also pipeline the extended-query protocol:
//! `Parse / Bind / Describe / Execute` are buffered until `Sync` (or the
//! configured `max_in_flight`), and forwarded as a single flush to the
//! backend so latency is dominated by RTT amortized across the batch.

use crate::{PoolRuntimeError, ProtocolPipelinePolicy};
use std::error::Error;
use std::fmt;

/// One pgwire extended-query frame the pool will buffer + forward.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExtendedFrame {
    Parse {
        statement_name: String,
        query: String,
    },
    Bind {
        portal_name: String,
        statement_name: String,
    },
    Describe {
        target: DescribeTarget,
        name: String,
    },
    Execute {
        portal_name: String,
        max_rows: i32,
    },
    Sync,
    Flush,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DescribeTarget {
    Portal,
    Statement,
}

impl ExtendedFrame {
    /// The wire tag this frame would serialize to, useful for log output.
    pub fn wire_tag(&self) -> char {
        match self {
            Self::Parse { .. } => 'P',
            Self::Bind { .. } => 'B',
            Self::Describe { .. } => 'D',
            Self::Execute { .. } => 'E',
            Self::Sync => 'S',
            Self::Flush => 'H',
        }
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

    /// Append a frame. If the frame is `Sync` (or the buffer is at the policy
    /// limit) the buffer is returned for the proxy to flush to the backend.
    pub fn append(
        &mut self,
        frame: ExtendedFrame,
    ) -> Result<Option<Vec<ExtendedFrame>>, PipelineError> {
        if matches!(frame, ExtendedFrame::Sync | ExtendedFrame::Flush) {
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
}

impl fmt::Display for PipelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow { max_in_flight } => write!(
                formatter,
                "pipeline buffer exceeded max_in_flight={max_in_flight}"
            ),
            Self::Runtime(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for PipelineError {}

impl From<PoolRuntimeError> for PipelineError {
    fn from(error: PoolRuntimeError) -> Self {
        Self::Runtime(error)
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
            .append(ExtendedFrame::Parse {
                statement_name: "s1".to_string(),
                query: "SELECT 1".to_string(),
            })
            .expect("parse");
        let _ = buffer
            .append(ExtendedFrame::Bind {
                portal_name: "p1".to_string(),
                statement_name: "s1".to_string(),
            })
            .expect("bind");
        let _ = buffer
            .append(ExtendedFrame::Execute {
                portal_name: "p1".to_string(),
                max_rows: 0,
            })
            .expect("execute");

        let flush = buffer
            .append(ExtendedFrame::Sync)
            .expect("sync")
            .expect("flush");

        assert_eq!(flush.len(), 4);
        assert_eq!(flush[0].wire_tag(), 'P');
        assert_eq!(flush[3].wire_tag(), 'S');
        assert_eq!(buffer.in_flight(), 0);
        assert_eq!(buffer.flushed_batches(), 1);
    }

    #[test]
    fn reaching_max_in_flight_auto_flushes() {
        let mut buffer = ExtendedPipelineBuffer::new(&policy(2)).expect("buffer");
        assert!(buffer
            .append(ExtendedFrame::Parse {
                statement_name: "s1".to_string(),
                query: "SELECT 1".to_string(),
            })
            .expect("parse")
            .is_none());
        let flush = buffer
            .append(ExtendedFrame::Bind {
                portal_name: "p1".to_string(),
                statement_name: "s1".to_string(),
            })
            .expect("bind");
        let frames = flush.expect("autoflush");
        assert_eq!(frames.len(), 2);
        assert_eq!(buffer.in_flight(), 0);
    }

    #[test]
    fn flush_returns_for_explicit_flush_frame() {
        let mut buffer = ExtendedPipelineBuffer::new(&policy(4)).expect("buffer");
        let _ = buffer
            .append(ExtendedFrame::Parse {
                statement_name: "s1".to_string(),
                query: "SELECT 1".to_string(),
            })
            .expect("parse");
        let frames = buffer
            .append(ExtendedFrame::Flush)
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
}
