// FEATURE: T7
// Derived from jackc/pgx pgproto3 (MIT).

//! Low-level read/write helpers used by every message implementation.
//!
//! The codec works on plain byte slices and `Vec<u8>` so the pool's proxy hot
//! path keeps its existing buffer types. `PgReader` tracks a cursor into a
//! borrowed `&[u8]`; `PgWriteBuf` is a thin wrapper around `Vec<u8>` that
//! exposes big-endian integer helpers and cstring framing.

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WireError {
    /// Body declared more bytes than the reader has remaining.
    Underflow { wanted: usize, remaining: usize },
    /// A cstring field was missing its trailing NUL terminator.
    UnterminatedCString { field: &'static str },
    /// A length-prefixed field claimed more bytes than fit in the body.
    InvalidLength { field: &'static str, value: i32 },
    /// A field that should be UTF-8 contained invalid sequences. The peer
    /// is technically allowed to send non-UTF-8 client_encoding, but the
    /// fields the pool inspects (statement_name, portal_name, query text,
    /// error fields) are always UTF-8 in practice.
    InvalidUtf8 { field: &'static str },
    /// A message tag byte did not match the expected value.
    UnexpectedTag { wanted: u8, got: u8 },
    /// The total declared length exceeds the configured maximum.
    MessageTooLarge { limit: usize, declared: usize },
    /// A startup envelope code was not one of the known magic values.
    UnknownStartupCode { code: u32 },
    /// Reserved value (e.g. ReadyForQuery transaction status byte).
    InvalidEnumValue { field: &'static str, value: u8 },
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Underflow { wanted, remaining } => {
                write!(f, "needed {wanted} more bytes, only {remaining} remain")
            }
            Self::UnterminatedCString { field } => {
                write!(f, "cstring field `{field}` missing NUL terminator")
            }
            Self::InvalidLength { field, value } => {
                write!(f, "length field `{field}` invalid: {value}")
            }
            Self::InvalidUtf8 { field } => {
                write!(f, "field `{field}` is not valid UTF-8")
            }
            Self::UnexpectedTag { wanted, got } => write!(
                f,
                "wanted message tag 0x{wanted:02x} (`{wanted_ch}`), got 0x{got:02x}",
                wanted_ch = char::from(*wanted)
            ),
            Self::MessageTooLarge { limit, declared } => {
                write!(f, "declared body length {declared} exceeds limit {limit}")
            }
            Self::UnknownStartupCode { code } => {
                write!(f, "startup envelope code {code} is not recognized")
            }
            Self::InvalidEnumValue { field, value } => {
                write!(f, "field `{field}` has invalid enum value 0x{value:02x}")
            }
        }
    }
}

impl Error for WireError {}

/// Cursor-style reader over a borrowed message body.
#[derive(Debug)]
pub struct PgReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> PgReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn finished(&self) -> bool {
        self.pos >= self.buf.len()
    }

    fn ensure(&self, n: usize) -> Result<(), WireError> {
        if self.remaining() < n {
            return Err(WireError::Underflow {
                wanted: n,
                remaining: self.remaining(),
            });
        }
        Ok(())
    }

    pub fn read_u8(&mut self) -> Result<u8, WireError> {
        self.ensure(1)?;
        let value = self.buf[self.pos];
        self.pos += 1;
        Ok(value)
    }

    pub fn read_i16(&mut self) -> Result<i16, WireError> {
        self.ensure(2)?;
        let bytes: [u8; 2] = self.buf[self.pos..self.pos + 2].try_into().unwrap();
        self.pos += 2;
        Ok(i16::from_be_bytes(bytes))
    }

    pub fn read_i32(&mut self) -> Result<i32, WireError> {
        self.ensure(4)?;
        let bytes: [u8; 4] = self.buf[self.pos..self.pos + 4].try_into().unwrap();
        self.pos += 4;
        Ok(i32::from_be_bytes(bytes))
    }

    pub fn read_u32(&mut self) -> Result<u32, WireError> {
        self.read_i32().map(|value| value as u32)
    }

    /// Read up to and including the next NUL terminator, returning the bytes
    /// before the NUL.
    pub fn read_cstring(&mut self, field: &'static str) -> Result<&'a [u8], WireError> {
        let start = self.pos;
        let nul = self.buf[start..]
            .iter()
            .position(|&byte| byte == 0)
            .ok_or(WireError::UnterminatedCString { field })?;
        let slice = &self.buf[start..start + nul];
        self.pos = start + nul + 1;
        Ok(slice)
    }

    /// Same as `read_cstring` but validates the bytes are UTF-8 and returns
    /// a `&str`. The fields the pool inspects (statement_name, query text,
    /// error message) are required to be UTF-8 for the pool's logging path.
    pub fn read_cstring_utf8(&mut self, field: &'static str) -> Result<&'a str, WireError> {
        let bytes = self.read_cstring(field)?;
        std::str::from_utf8(bytes).map_err(|_| WireError::InvalidUtf8 { field })
    }

    /// Read the next `n` bytes without copying.
    pub fn read_slice(&mut self, n: usize) -> Result<&'a [u8], WireError> {
        self.ensure(n)?;
        let slice = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    /// Consume the remaining body unchanged.
    pub fn read_to_end(&mut self) -> &'a [u8] {
        let slice = &self.buf[self.pos..];
        self.pos = self.buf.len();
        slice
    }
}

/// Append-only buffer with helpers for the codec's encode path.
#[derive(Debug, Default)]
pub struct PgWriteBuf {
    buf: Vec<u8>,
}

impl PgWriteBuf {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
        }
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.buf
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn write_u8(&mut self, value: u8) {
        self.buf.push(value);
    }

    pub fn write_i16(&mut self, value: i16) {
        self.buf.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_i32(&mut self, value: i32) {
        self.buf.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_u32(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        self.buf.extend_from_slice(value);
    }

    /// Append `value` and a trailing NUL.
    pub fn write_cstring(&mut self, value: &[u8]) {
        self.buf.extend_from_slice(value);
        self.buf.push(0);
    }

    /// Append `value` (UTF-8) and a trailing NUL.
    pub fn write_cstring_str(&mut self, value: &str) {
        self.write_cstring(value.as_bytes());
    }

    /// Reserve a length-prefix slot, run `body`, then back-patch the slot
    /// with the byte count written (including the 4-byte slot itself).
    pub fn write_length_prefixed<F: FnOnce(&mut Self)>(&mut self, body: F) {
        let start = self.buf.len();
        self.write_i32(0);
        body(self);
        let end = self.buf.len();
        let length = (end - start) as u32;
        self.buf[start..start + 4].copy_from_slice(&length.to_be_bytes());
    }

    /// Write a complete tagged frame: `[tag][i32 length][body]`. The length
    /// covers the i32 itself and the body.
    pub fn write_tagged_frame<F: FnOnce(&mut Self)>(&mut self, tag: u8, body: F) {
        self.write_u8(tag);
        self.write_length_prefixed(body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cstring_roundtrip() {
        let mut buf = PgWriteBuf::new();
        buf.write_cstring_str("hello");
        let bytes = buf.into_inner();
        assert_eq!(bytes, b"hello\0");

        let mut reader = PgReader::new(&bytes);
        assert_eq!(reader.read_cstring_utf8("greeting").unwrap(), "hello");
        assert!(reader.finished());
    }

    #[test]
    fn tagged_frame_back_patches_length() {
        let mut buf = PgWriteBuf::new();
        buf.write_tagged_frame(b'P', |body| {
            body.write_cstring_str("");
            body.write_cstring_str("SELECT 1");
            body.write_i16(0);
        });
        let bytes = buf.into_inner();
        assert_eq!(bytes[0], b'P');
        let length = u32::from_be_bytes(bytes[1..5].try_into().unwrap());
        assert_eq!(length as usize, bytes.len() - 1);
    }

    #[test]
    fn cstring_without_nul_errors() {
        let bytes = b"no-terminator";
        let mut reader = PgReader::new(bytes);
        assert!(matches!(
            reader.read_cstring_utf8("field"),
            Err(WireError::UnterminatedCString { field: "field" })
        ));
    }

    #[test]
    fn underflow_reports_remaining() {
        let bytes = b"\x00\x01";
        let mut reader = PgReader::new(bytes);
        assert!(matches!(
            reader.read_i32(),
            Err(WireError::Underflow {
                wanted: 4,
                remaining: 2
            })
        ));
    }
}
