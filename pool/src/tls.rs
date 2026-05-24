// FEATURE: R10

//! TLS session-ticket key rotation.
//!
//! This module validates ticket-key material, keeps a bounded current/previous
//! key ring, and emits non-secret rotation evidence. It does not claim live
//! rustls listener integration, external Secret mounting, or production TLS
//! infrastructure.

use crate::{PoolRuntimeError, TlsSessionTicketPolicy};
use std::error::Error;
use std::fmt;
use std::time::{Duration, SystemTime};

/// Fixed-size ticket key. rustls's ticketer expects 32+ bytes.
pub const TICKET_KEY_LEN: usize = 32;

#[derive(Clone, Eq, PartialEq)]
pub struct TicketKey {
    pub material: [u8; TICKET_KEY_LEN],
    pub created_at: SystemTime,
}

impl TicketKey {
    pub fn new(material: [u8; TICKET_KEY_LEN], created_at: SystemTime) -> Self {
        Self {
            material,
            created_at,
        }
    }

    /// Construct a key from a hex-encoded string (64 hex chars).
    pub fn from_hex(input: &str, created_at: SystemTime) -> Result<Self, TlsTicketError> {
        if input.len() != TICKET_KEY_LEN * 2 {
            return Err(TlsTicketError::InvalidKeyLength(input.len()));
        }
        if !input.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(TlsTicketError::InvalidHex);
        }
        let mut material = [0_u8; TICKET_KEY_LEN];
        for index in 0..TICKET_KEY_LEN {
            let byte = u8::from_str_radix(&input[index * 2..index * 2 + 2], 16)
                .map_err(|_| TlsTicketError::InvalidHex)?;
            material[index] = byte;
        }
        Ok(Self::new(material, created_at))
    }

    pub fn redacted_fingerprint(&self) -> String {
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET;
        for byte in self.material {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        format!("{hash:016x}")
    }
}

impl fmt::Debug for TicketKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TicketKey")
            .field("material", &"<redacted>")
            .field("fingerprint", &self.redacted_fingerprint())
            .field("created_at", &self.created_at)
            .finish()
    }
}

/// Ring of (current, previous) ticket keys. The previous key remains valid
/// for one rotation period so resumption survives a key rotation; everything
/// beyond that is rejected and the client renegotiates.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TicketKeyRing {
    pub current: TicketKey,
    pub previous: Option<TicketKey>,
    pub rotation: Duration,
}

impl TicketKeyRing {
    pub fn new(current: TicketKey, rotation: Duration) -> Result<Self, TlsTicketError> {
        if rotation.is_zero() {
            return Err(TlsTicketError::InvalidRotation);
        }
        Ok(Self {
            current,
            previous: None,
            rotation,
        })
    }

    /// Rotate the ring: move `current -> previous` and install `new_key` as
    /// `current`. Older `previous` keys are forgotten.
    pub fn rotate(&mut self, new_key: TicketKey) {
        self.previous = Some(std::mem::replace(&mut self.current, new_key));
    }

    /// Check whether the supplied wall-clock time falls within either key's
    /// validity window. Used by tests + admin probes.
    pub fn validates(&self, now: SystemTime) -> bool {
        self.accepts_current(now) || self.accepts_previous(now)
    }

    pub fn accepts_current(&self, now: SystemTime) -> bool {
        self.is_within_window(&self.current, now)
    }

    pub fn accepts_previous(&self, now: SystemTime) -> bool {
        self.previous
            .as_ref()
            .map(|previous| self.is_within_window(previous, now))
            .unwrap_or(false)
    }

    fn is_within_window(&self, key: &TicketKey, now: SystemTime) -> bool {
        let window = self.rotation * 2;
        now.duration_since(key.created_at)
            .map(|elapsed| elapsed <= window)
            .unwrap_or(false)
    }

    /// Returns whether `now` is past the configured rotation interval since
    /// `current.created_at` and a rotation should run.
    pub fn rotation_due(&self, now: SystemTime) -> bool {
        now.duration_since(self.current.created_at)
            .map(|elapsed| elapsed >= self.rotation)
            .unwrap_or(false)
    }

    pub fn rotation_report(&self, now: SystemTime) -> TicketRotationReport {
        TicketRotationReport {
            rotation_seconds: self.rotation.as_secs(),
            rotation_due: self.rotation_due(now),
            current_key_accepted: self.accepts_current(now),
            previous_key_present: self.previous.is_some(),
            previous_key_accepted: self.accepts_previous(now),
            current_key_fingerprint: self.current.redacted_fingerprint(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TicketRotationReport {
    pub rotation_seconds: u64,
    pub rotation_due: bool,
    pub current_key_accepted: bool,
    pub previous_key_present: bool,
    pub previous_key_accepted: bool,
    pub current_key_fingerprint: String,
}

/// Construct a ticket-key ring from policy + a hex-encoded material string.
pub fn ring_from_policy(
    policy: &TlsSessionTicketPolicy,
    hex_material: &str,
) -> Result<TicketKeyRing, TlsTicketError> {
    ring_from_policy_at(policy, hex_material, SystemTime::now())
}

pub fn ring_from_policy_at(
    policy: &TlsSessionTicketPolicy,
    hex_material: &str,
    created_at: SystemTime,
) -> Result<TicketKeyRing, TlsTicketError> {
    policy.validate().map_err(TlsTicketError::Runtime)?;
    if !policy.enabled {
        return Err(TlsTicketError::Disabled);
    }
    let key = TicketKey::from_hex(hex_material, created_at)?;
    TicketKeyRing::new(key, Duration::from_secs(policy.rotation_seconds as u64))
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TlsTicketError {
    Disabled,
    InvalidHex,
    InvalidKeyLength(usize),
    InvalidRotation,
    Runtime(PoolRuntimeError),
}

impl fmt::Display for TlsTicketError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => write!(formatter, "TLS ticket reuse is disabled"),
            Self::InvalidHex => write!(formatter, "ticket key contains non-hex characters"),
            Self::InvalidKeyLength(length) => {
                write!(formatter, "ticket key hex must be 64 chars, got {length}")
            }
            Self::InvalidRotation => write!(formatter, "rotation must be greater than zero"),
            Self::Runtime(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for TlsTicketError {}

impl From<PoolRuntimeError> for TlsTicketError {
    fn from(error: PoolRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn current_key(offset: Duration) -> TicketKey {
        TicketKey::from_hex(SAMPLE_HEX, SystemTime::now() - offset).expect("key")
    }

    #[test]
    fn from_hex_accepts_64_chars() {
        let key = TicketKey::from_hex(SAMPLE_HEX, SystemTime::now()).expect("key");
        assert_eq!(key.material[0], 0x01);
        assert_eq!(key.material[1], 0x23);
        assert_eq!(key.material[31], 0xef);
    }

    #[test]
    fn from_hex_rejects_short_input() {
        assert!(matches!(
            TicketKey::from_hex("00", SystemTime::now()),
            Err(TlsTicketError::InvalidKeyLength(2))
        ));
    }

    #[test]
    fn from_hex_rejects_non_hex() {
        let bad = "z".repeat(64);
        assert_eq!(
            TicketKey::from_hex(&bad, SystemTime::now()),
            Err(TlsTicketError::InvalidHex)
        );
    }

    #[test]
    fn redacted_fingerprint_is_stable_and_secret_free() {
        let key = TicketKey::from_hex(SAMPLE_HEX, SystemTime::now()).expect("key");
        let fingerprint = key.redacted_fingerprint();
        assert_eq!(fingerprint.len(), 16);
        assert!(!fingerprint.contains("012345"));
    }

    #[test]
    fn rotation_zero_rejected() {
        let result = TicketKeyRing::new(current_key(Duration::ZERO), Duration::ZERO);
        assert!(matches!(result, Err(TlsTicketError::InvalidRotation)));
    }

    #[test]
    fn rotate_keeps_previous() {
        let mut ring =
            TicketKeyRing::new(current_key(Duration::ZERO), Duration::from_secs(60)).expect("ring");
        let next = current_key(Duration::ZERO);
        ring.rotate(next.clone());
        assert_eq!(ring.current, next);
        assert!(ring.previous.is_some());
    }

    #[test]
    fn rotation_due_after_interval() {
        let ring = TicketKeyRing::new(
            current_key(Duration::from_secs(120)),
            Duration::from_secs(60),
        )
        .expect("ring");
        assert!(ring.rotation_due(SystemTime::now()));
    }

    #[test]
    fn validates_uses_current_and_previous() {
        let mut ring =
            TicketKeyRing::new(current_key(Duration::ZERO), Duration::from_secs(60)).expect("ring");
        let new_key = TicketKey::from_hex(SAMPLE_HEX, SystemTime::now()).expect("new key");
        ring.rotate(new_key);
        assert!(ring.validates(SystemTime::now()));
    }

    #[test]
    fn ring_from_policy_requires_enabled() {
        let policy = TlsSessionTicketPolicy {
            enabled: false,
            rotation_seconds: 60,
        };
        assert!(matches!(
            ring_from_policy(&policy, SAMPLE_HEX),
            Err(TlsTicketError::Disabled)
        ));
    }

    #[test]
    fn rotation_report_keeps_key_material_redacted() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let mut ring = ring_from_policy_at(
            &TlsSessionTicketPolicy {
                enabled: true,
                rotation_seconds: 60,
            },
            SAMPLE_HEX,
            now - Duration::from_secs(61),
        )
        .expect("ring");
        assert!(ring.rotation_due(now));
        ring.rotate(TicketKey::from_hex(SAMPLE_HEX, now).expect("key"));
        let report = ring.rotation_report(now);
        assert!(report.current_key_accepted);
        assert!(report.previous_key_present);
        assert!(report.previous_key_accepted);
        assert_eq!(report.current_key_fingerprint.len(), 16);
    }
}
