// FEATURE: T7

//! Startup-phase envelopes for the PostgreSQL v3 wire protocol.
//!
//! Unlike regular tagged frames, the very first envelope a client sends has
//! no tag byte:
//!
//! ```text
//!     [u32 length][u32 code][body ...]
//! ```
//!
//! `code` is either a 16/16 split protocol version (0x00030000 == 3.0) or one
//! of the special magic numbers SSLRequest, GSSENCRequest, CancelRequest.
//!
//! The pool already buffers the entire envelope in
//! `pool/src/trace_tap.rs::tap_startup_message`. This module re-expresses the
//! parse/encode path on top of `PgReader` / `PgWriteBuf` so the same codec
//! types power the rest of the proxy.

use crate::codec::{PgReader, PgWriteBuf, WireError};
use crate::MAX_STARTUP_ENVELOPE_BYTES;

pub const PROTOCOL_VERSION_3_0: u32 = 0x0003_0000;
pub const CANCEL_REQUEST_CODE: u32 = 80_877_102;
pub const SSL_REQUEST_CODE: u32 = 80_877_103;
pub const GSSENC_REQUEST_CODE: u32 = 80_877_104;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StartupEnvelope {
    Startup(StartupMessage),
    Cancel(CancelRequest),
    Ssl(SslRequest),
    GssEnc(GssEncRequest),
}

impl StartupEnvelope {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        match self {
            Self::Startup(message) => message.encode(buf),
            Self::Cancel(request) => request.encode(buf),
            Self::Ssl(request) => request.encode(buf),
            Self::GssEnc(request) => request.encode(buf),
        }
    }

    /// Parse a complete startup envelope. The caller is responsible for
    /// having read at least `MAX_STARTUP_ENVELOPE_BYTES` worth of buffer or
    /// the entire envelope (whichever is smaller) before calling.
    pub fn decode(buf: &[u8]) -> Result<Self, WireError> {
        if buf.len() < 8 {
            return Err(WireError::Underflow {
                wanted: 8,
                remaining: buf.len(),
            });
        }
        let length = u32::from_be_bytes(buf[0..4].try_into().unwrap()) as usize;
        if length < 8 {
            return Err(WireError::InvalidLength {
                field: "startup-length",
                value: length as i32,
            });
        }
        if length > MAX_STARTUP_ENVELOPE_BYTES {
            return Err(WireError::MessageTooLarge {
                limit: MAX_STARTUP_ENVELOPE_BYTES,
                declared: length,
            });
        }
        if buf.len() < length {
            return Err(WireError::Underflow {
                wanted: length,
                remaining: buf.len(),
            });
        }
        let code = u32::from_be_bytes(buf[4..8].try_into().unwrap());
        match code {
            CANCEL_REQUEST_CODE => CancelRequest::decode_body(&buf[8..length]).map(Self::Cancel),
            SSL_REQUEST_CODE => Ok(Self::Ssl(SslRequest)),
            GSSENC_REQUEST_CODE => Ok(Self::GssEnc(GssEncRequest)),
            version if (version >> 16) >= 2 && (version >> 16) <= 4 => {
                StartupMessage::decode_body(version, &buf[8..length]).map(Self::Startup)
            }
            unknown => Err(WireError::UnknownStartupCode { code: unknown }),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StartupMessage {
    pub protocol_version: u32,
    pub parameters: Vec<(String, String)>,
}

impl StartupMessage {
    pub fn new(parameters: Vec<(String, String)>) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION_3_0,
            parameters,
        }
    }

    pub fn parameter(&self, name: &str) -> Option<&str> {
        self.parameters
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_length_prefixed(|body| {
            body.write_u32(self.protocol_version);
            for (key, value) in &self.parameters {
                body.write_cstring_str(key);
                body.write_cstring_str(value);
            }
            body.write_u8(0);
        });
    }

    fn decode_body(version: u32, body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let mut parameters = Vec::new();
        while reader.remaining() > 0 {
            let next = reader.read_u8()?;
            if next == 0 {
                break;
            }
            // The key cstring starts at `next` (so we have to step back one
            // byte before reading the rest of the cstring). Implement it by
            // hand to keep `PgReader` simple.
            let key = decode_startup_cstring(next, &mut reader, "startup-parameter-key")?;
            let value_first = reader.read_u8()?;
            let value = decode_startup_cstring(
                value_first,
                &mut reader,
                "startup-parameter-value",
            )?;
            parameters.push((key, value));
        }
        Ok(Self {
            protocol_version: version,
            parameters,
        })
    }
}

fn decode_startup_cstring(
    first_byte: u8,
    reader: &mut PgReader<'_>,
    field: &'static str,
) -> Result<String, WireError> {
    if first_byte == 0 {
        return Ok(String::new());
    }
    // The cstring continues until a NUL terminator.
    let mut bytes = vec![first_byte];
    loop {
        let next = reader.read_u8()?;
        if next == 0 {
            break;
        }
        bytes.push(next);
    }
    String::from_utf8(bytes).map_err(|_| WireError::InvalidUtf8 { field })
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CancelRequest {
    pub process_id: i32,
    pub secret_key: i32,
}

impl CancelRequest {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_length_prefixed(|body| {
            body.write_u32(CANCEL_REQUEST_CODE);
            body.write_i32(self.process_id);
            body.write_i32(self.secret_key);
        });
    }

    fn decode_body(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let process_id = reader.read_i32()?;
        let secret_key = reader.read_i32()?;
        Ok(Self {
            process_id,
            secret_key,
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct SslRequest;

impl SslRequest {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_length_prefixed(|body| {
            body.write_u32(SSL_REQUEST_CODE);
        });
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct GssEncRequest;

impl GssEncRequest {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_length_prefixed(|body| {
            body.write_u32(GSSENC_REQUEST_CODE);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(envelope: StartupEnvelope) -> StartupEnvelope {
        let mut buf = PgWriteBuf::new();
        envelope.encode(&mut buf);
        let bytes = buf.into_inner();
        StartupEnvelope::decode(&bytes).expect("decode")
    }

    #[test]
    fn startup_message_with_libpq_parameters() {
        let original = StartupEnvelope::Startup(StartupMessage::new(vec![
            ("user".to_string(), "tenant_a".to_string()),
            ("database".to_string(), "postgres".to_string()),
            ("application_name".to_string(), "ai-blaise-test".to_string()),
        ]));
        let round = roundtrip(original.clone());
        assert_eq!(round, original);
        if let StartupEnvelope::Startup(message) = round {
            assert_eq!(message.parameter("user"), Some("tenant_a"));
            assert_eq!(message.parameter("application_name"), Some("ai-blaise-test"));
        } else {
            unreachable!()
        }
    }

    #[test]
    fn cancel_request_roundtrip() {
        let original = StartupEnvelope::Cancel(CancelRequest {
            process_id: 4242,
            secret_key: 0x4001,
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn ssl_request_envelope_is_eight_bytes() {
        let mut buf = PgWriteBuf::new();
        StartupEnvelope::Ssl(SslRequest).encode(&mut buf);
        let bytes = buf.into_inner();
        assert_eq!(bytes.len(), 8);
        assert_eq!(
            u32::from_be_bytes(bytes[0..4].try_into().unwrap()),
            8
        );
        assert_eq!(
            u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
            SSL_REQUEST_CODE
        );
    }

    #[test]
    fn gssenc_request_envelope_is_eight_bytes() {
        let mut buf = PgWriteBuf::new();
        StartupEnvelope::GssEnc(GssEncRequest).encode(&mut buf);
        let bytes = buf.into_inner();
        assert_eq!(bytes.len(), 8);
        assert_eq!(
            u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
            GSSENC_REQUEST_CODE
        );
    }

    #[test]
    fn rejects_oversize_envelope() {
        let mut buf = vec![];
        buf.extend_from_slice(&((MAX_STARTUP_ENVELOPE_BYTES as u32) + 1).to_be_bytes());
        buf.extend_from_slice(&PROTOCOL_VERSION_3_0.to_be_bytes());
        assert!(matches!(
            StartupEnvelope::decode(&buf),
            Err(WireError::MessageTooLarge { .. })
        ));
    }

    #[test]
    fn rejects_unknown_envelope_code() {
        let mut buf = vec![];
        buf.extend_from_slice(&12_u32.to_be_bytes());
        buf.extend_from_slice(&0x00FF_0000_u32.to_be_bytes());
        buf.extend_from_slice(&[0, 0, 0, 0]);
        assert!(matches!(
            StartupEnvelope::decode(&buf),
            Err(WireError::UnknownStartupCode { .. })
        ));
    }
}
