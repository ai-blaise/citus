// FEATURE: T3

//! Virtual PID multiplexing for `pg_cancel_backend` routing.
//!
//! Clients see a pool-issued virtual PID rather than the real backend PID. The
//! pool intercepts cancel requests carrying the virtual PID, looks up the real
//! backend, and forwards a cancel keyed on the real PID + secret. This keeps
//! the upstream cancel keys completely opaque to clients while still letting
//! the pgwire `CancelRequest` message work.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};

const VIRTUAL_PID_RESERVED: u32 = 1_000;
const VIRTUAL_PID_MAX: u32 = i32::MAX as u32;

/// Real backend identity referenced by a virtual PID.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealBackend {
    pub backend_id: String,
    pub real_pid: i32,
    pub cancel_key: i32,
    pub host: String,
    pub port: u16,
}

impl RealBackend {
    pub fn validate(&self) -> Result<(), VirtualPidError> {
        if self.backend_id.trim().is_empty() {
            return Err(VirtualPidError::MissingField("backend_id"));
        }
        if self.host.trim().is_empty() {
            return Err(VirtualPidError::MissingField("host"));
        }
        if self.port == 0 {
            return Err(VirtualPidError::InvalidPort);
        }
        if self.real_pid <= 0 {
            return Err(VirtualPidError::InvalidRealPid);
        }
        Ok(())
    }
}

/// Mapping table from `virtual_pid -> RealBackend`.
///
/// Implemented with monotonically-increasing virtual PIDs starting at
/// `VIRTUAL_PID_RESERVED`. Cancel routing only consults this map; the pool
/// never sends the real PID over the wire.
#[derive(Debug)]
pub struct VirtualPidTable {
    next_virtual_pid: AtomicU32,
    by_virtual: std::sync::Mutex<BTreeMap<u32, RealBackend>>,
}

impl VirtualPidTable {
    pub fn new() -> Self {
        Self {
            next_virtual_pid: AtomicU32::new(VIRTUAL_PID_RESERVED),
            by_virtual: std::sync::Mutex::new(BTreeMap::new()),
        }
    }

    /// Allocate the next virtual PID and register it against `backend`.
    pub fn allocate(&self, backend: RealBackend) -> Result<u32, VirtualPidError> {
        backend.validate()?;
        let virtual_pid = self.next_virtual_pid.fetch_add(1, Ordering::Relaxed);
        if virtual_pid >= VIRTUAL_PID_MAX {
            return Err(VirtualPidError::PidSpaceExhausted);
        }
        let mut by_virtual = self
            .by_virtual
            .lock()
            .map_err(|_| VirtualPidError::PoisonedLock)?;
        by_virtual.insert(virtual_pid, backend);
        Ok(virtual_pid)
    }

    pub fn resolve(&self, virtual_pid: u32) -> Result<RealBackend, VirtualPidError> {
        let by_virtual = self
            .by_virtual
            .lock()
            .map_err(|_| VirtualPidError::PoisonedLock)?;
        by_virtual
            .get(&virtual_pid)
            .cloned()
            .ok_or(VirtualPidError::UnknownVirtualPid(virtual_pid))
    }

    pub fn release(&self, virtual_pid: u32) -> Result<RealBackend, VirtualPidError> {
        let mut by_virtual = self
            .by_virtual
            .lock()
            .map_err(|_| VirtualPidError::PoisonedLock)?;
        by_virtual
            .remove(&virtual_pid)
            .ok_or(VirtualPidError::UnknownVirtualPid(virtual_pid))
    }

    pub fn len(&self) -> usize {
        self.by_virtual
            .lock()
            .map(|map| map.len())
            .unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for VirtualPidTable {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum VirtualPidError {
    InvalidPort,
    InvalidRealPid,
    MissingField(&'static str),
    PidSpaceExhausted,
    PoisonedLock,
    UnknownVirtualPid(u32),
}

impl fmt::Display for VirtualPidError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPort => write!(formatter, "port must be greater than zero"),
            Self::InvalidRealPid => write!(formatter, "real_pid must be a positive int32"),
            Self::MissingField(field) => write!(formatter, "{field} must not be empty"),
            Self::PidSpaceExhausted => write!(formatter, "virtual PID space exhausted"),
            Self::PoisonedLock => write!(formatter, "virtual PID table lock poisoned"),
            Self::UnknownVirtualPid(virtual_pid) => {
                write!(formatter, "unknown virtual PID {virtual_pid}")
            }
        }
    }
}

impl Error for VirtualPidError {}

/// pgwire `CancelRequest` payload: 16 bytes, big-endian, with magic = 80877102.
///
/// Layout: `[u32 magic][u32 pid][u32 secret]`. Codec lives in the
/// `ai_blaise_citus_pool_wire` crate; this re-export preserves the historical
/// constant name for downstream consumers.
pub use ai_blaise_citus_pool_wire::CANCEL_REQUEST_CODE as PGWIRE_CANCEL_MAGIC;

/// Parse a cancel request, returning the virtual PID and the client-supplied
/// secret. Callers should compare the secret against the recorded
/// `cancel_key` before forwarding.
pub fn parse_cancel_request(bytes: &[u8]) -> Result<(u32, i32), VirtualPidError> {
    let envelope = match ai_blaise_citus_pool_wire::StartupEnvelope::decode(bytes) {
        Ok(envelope) => envelope,
        Err(ai_blaise_citus_pool_wire::WireError::UnknownStartupCode { .. }) => {
            return Err(VirtualPidError::MissingField("cancel-request magic"))
        }
        Err(_) => return Err(VirtualPidError::MissingField("cancel-request body")),
    };
    match envelope {
        ai_blaise_citus_pool_wire::StartupEnvelope::Cancel(request) => {
            Ok((request.process_id as u32, request.secret_key))
        }
        _ => Err(VirtualPidError::MissingField("cancel-request magic")),
    }
}

/// Encode the upstream-side cancel-request frame for the recorded real PID
/// and the stored cancel key.
pub fn encode_cancel_request(real_pid: i32, cancel_key: i32) -> Vec<u8> {
    let mut buf = ai_blaise_citus_pool_wire::PgWriteBuf::with_capacity(16);
    ai_blaise_citus_pool_wire::CancelRequest {
        process_id: real_pid,
        secret_key: cancel_key,
    }
    .encode(&mut buf);
    buf.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_backend(pid: i32) -> RealBackend {
        RealBackend {
            backend_id: format!("backend-{pid}"),
            real_pid: pid,
            cancel_key: pid + 1_000_000,
            host: "worker-a".to_string(),
            port: 5432,
        }
    }

    #[test]
    fn allocate_assigns_distinct_virtual_pids() {
        let table = VirtualPidTable::new();
        let first = table.allocate(sample_backend(101)).expect("alloc 1");
        let second = table.allocate(sample_backend(102)).expect("alloc 2");
        assert!(first >= VIRTUAL_PID_RESERVED);
        assert!(second > first);
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn resolve_returns_registered_backend() {
        let table = VirtualPidTable::new();
        let backend = sample_backend(555);
        let virtual_pid = table.allocate(backend.clone()).expect("alloc");
        assert_eq!(table.resolve(virtual_pid), Ok(backend));
    }

    #[test]
    fn unknown_virtual_pid_is_rejected() {
        let table = VirtualPidTable::new();
        assert_eq!(
            table.resolve(999_999),
            Err(VirtualPidError::UnknownVirtualPid(999_999))
        );
    }

    #[test]
    fn release_frees_the_slot() {
        let table = VirtualPidTable::new();
        let virtual_pid = table.allocate(sample_backend(7)).expect("alloc");
        let released = table.release(virtual_pid).expect("release");
        assert_eq!(released.real_pid, 7);
        assert!(table.resolve(virtual_pid).is_err());
    }

    #[test]
    fn invalid_backend_is_rejected() {
        let table = VirtualPidTable::new();
        let mut backend = sample_backend(7);
        backend.real_pid = 0;
        assert!(matches!(
            table.allocate(backend),
            Err(VirtualPidError::InvalidRealPid)
        ));
    }

    #[test]
    fn cancel_request_roundtrip() {
        let frame = encode_cancel_request(4242, 0x4001);
        let (pid, secret) = parse_cancel_request(&frame).expect("parse");
        assert_eq!(pid, 4242);
        assert_eq!(secret, 0x4001);
    }

    #[test]
    fn parse_cancel_request_rejects_wrong_magic() {
        let mut frame = encode_cancel_request(1, 1);
        frame[4] = 0xff;
        assert!(matches!(
            parse_cancel_request(&frame),
            Err(VirtualPidError::MissingField("cancel-request magic"))
        ));
    }
}
