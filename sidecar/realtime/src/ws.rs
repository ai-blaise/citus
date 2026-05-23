//! WebSocket server primitives implemented in pure `std`.
//!
//! Scope is intentionally minimal — RFC 6455 handshake + text-frame encode
//! + masked text-frame decode + close-frame encode — because the realtime
//! sidecar only speaks Phoenix-channel JSON text frames over the wire.
//! Compression, extensions, and binary frames are not implemented; the
//! handshake refuses any client that requires them.

// FEATURE: RT1
// FEATURE: RT2
// FEATURE: RT3
// FEATURE: RT4
// FEATURE: RT5

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;

const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Decoded HTTP upgrade request.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UpgradeRequest {
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
}

impl UpgradeRequest {
    pub fn parse(buffer: &[u8]) -> Result<Self, WsError> {
        let text = std::str::from_utf8(buffer).map_err(|_| WsError::InvalidHandshake)?;
        let mut lines = text.lines();
        let request_line = lines.next().ok_or(WsError::InvalidHandshake)?;
        let mut parts = request_line.split_whitespace();
        let _method = parts.next().ok_or(WsError::InvalidHandshake)?;
        let target = parts.next().ok_or(WsError::InvalidHandshake)?;
        let (path, query_string) = match target.find('?') {
            Some(index) => (target[..index].to_string(), &target[index + 1..]),
            None => (target.to_string(), ""),
        };
        let mut query = HashMap::new();
        for pair in query_string.split('&').filter(|item| !item.is_empty()) {
            let mut split = pair.splitn(2, '=');
            let key = split.next().unwrap_or_default().to_string();
            let value = split.next().unwrap_or_default().to_string();
            query.insert(key, value);
        }
        let mut headers = HashMap::new();
        for line in lines {
            if line.is_empty() {
                break;
            }
            let mut split = line.splitn(2, ':');
            let key = split
                .next()
                .ok_or(WsError::InvalidHandshake)?
                .trim()
                .to_ascii_lowercase();
            let value = split
                .next()
                .ok_or(WsError::InvalidHandshake)?
                .trim()
                .to_string();
            headers.insert(key, value);
        }
        Ok(Self {
            path,
            query,
            headers,
        })
    }

    /// Compute the `Sec-WebSocket-Accept` header value the server must
    /// return.
    pub fn accept_key(&self) -> Result<String, WsError> {
        let key = self
            .headers
            .get("sec-websocket-key")
            .ok_or(WsError::MissingKey)?;
        let mut input = String::with_capacity(key.len() + WS_GUID.len());
        input.push_str(key);
        input.push_str(WS_GUID);
        Ok(base64_encode(&sha1(input.as_bytes())))
    }

    pub fn handshake_response(&self) -> Result<Vec<u8>, WsError> {
        let accept = self.accept_key()?;
        let body = format!(
            "HTTP/1.1 101 Switching Protocols\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Accept: {accept}\r\n\r\n"
        );
        Ok(body.into_bytes())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WsError {
    InvalidHandshake,
    MissingKey,
    InvalidFrame,
    Closed,
    UnsupportedOpcode(u8),
}

impl std::fmt::Display for WsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHandshake => write!(formatter, "ws handshake malformed"),
            Self::MissingKey => write!(formatter, "ws handshake missing Sec-WebSocket-Key"),
            Self::InvalidFrame => write!(formatter, "ws frame malformed"),
            Self::Closed => write!(formatter, "ws connection closed"),
            Self::UnsupportedOpcode(op) => write!(formatter, "ws unsupported opcode {op:#x}"),
        }
    }
}

impl std::error::Error for WsError {}

/// Encode a text frame from the server (no masking).
pub fn encode_text_frame(text: &str) -> Vec<u8> {
    encode_frame(0x1, text.as_bytes())
}

/// Encode a close frame.
pub fn encode_close_frame(code: u16, reason: &str) -> Vec<u8> {
    let mut payload = Vec::with_capacity(2 + reason.len());
    payload.extend_from_slice(&code.to_be_bytes());
    payload.extend_from_slice(reason.as_bytes());
    encode_frame(0x8, &payload)
}

/// Decode one frame from the buffer. Returns the opcode and payload bytes.
/// Returns `Ok(None)` when more bytes are needed.
pub fn decode_frame(buffer: &[u8]) -> Result<Option<(u8, Vec<u8>, usize)>, WsError> {
    if buffer.len() < 2 {
        return Ok(None);
    }
    let opcode = buffer[0] & 0x0F;
    let masked = buffer[1] & 0x80 != 0;
    let mut payload_len = (buffer[1] & 0x7F) as usize;
    let mut header_size = 2;
    if payload_len == 126 {
        if buffer.len() < 4 {
            return Ok(None);
        }
        payload_len = u16::from_be_bytes([buffer[2], buffer[3]]) as usize;
        header_size = 4;
    } else if payload_len == 127 {
        if buffer.len() < 10 {
            return Ok(None);
        }
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(&buffer[2..10]);
        payload_len = u64::from_be_bytes(bytes) as usize;
        header_size = 10;
    }
    let mask_size = if masked { 4 } else { 0 };
    if buffer.len() < header_size + mask_size + payload_len {
        return Ok(None);
    }
    let payload_start = header_size + mask_size;
    let mut payload = buffer[payload_start..payload_start + payload_len].to_vec();
    if masked {
        let mask = &buffer[header_size..header_size + 4];
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
    }
    let consumed = payload_start + payload_len;
    Ok(Some((opcode, payload, consumed)))
}

/// Convenience wrapper for the WS connection state.
#[derive(Debug)]
pub struct WsConnection {
    pub stream: TcpStream,
    pub buffer: Vec<u8>,
    pub remote: String,
}

impl WsConnection {
    pub fn new(stream: TcpStream) -> Self {
        let remote = stream
            .peer_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        Self {
            stream,
            buffer: Vec::with_capacity(4096),
            remote,
        }
    }

    /// Pull bytes from the socket into the connection buffer. Returns `0`
    /// when the connection is closed.
    pub fn pull(&mut self) -> std::io::Result<usize> {
        let mut chunk = [0_u8; 1024];
        match self.stream.read(&mut chunk) {
            Ok(n) => {
                self.buffer.extend_from_slice(&chunk[..n]);
                Ok(n)
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut =>
            {
                Ok(0)
            }
            Err(error) => Err(error),
        }
    }

    pub fn write_text(&mut self, text: &str) -> std::io::Result<()> {
        let frame = encode_text_frame(text);
        self.stream.write_all(&frame)?;
        self.stream.flush()
    }

    pub fn write_close(&mut self, code: u16, reason: &str) -> std::io::Result<()> {
        let frame = encode_close_frame(code, reason);
        self.stream.write_all(&frame)?;
        self.stream.flush()
    }

    /// Drain one frame from the buffer if one is available.
    pub fn next_text_frame(&mut self) -> Result<Option<String>, WsError> {
        loop {
            let Some((opcode, payload, consumed)) = decode_frame(&self.buffer)? else {
                return Ok(None);
            };
            self.buffer.drain(..consumed);
            match opcode {
                0x1 => {
                    let text = String::from_utf8(payload).map_err(|_| WsError::InvalidFrame)?;
                    return Ok(Some(text));
                }
                0x9 => {
                    // ping -> respond with pong
                    let pong = encode_frame(0xA, &payload);
                    self.stream.write_all(&pong).map_err(|_| WsError::Closed)?;
                }
                0xA => {} // pong, ignore
                0x8 => return Err(WsError::Closed),
                other => return Err(WsError::UnsupportedOpcode(other)),
            }
        }
    }
}

fn encode_frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 14);
    frame.push(0x80 | (opcode & 0x0F)); // FIN + opcode
    if payload.len() < 126 {
        frame.push(payload.len() as u8);
    } else if payload.len() < (1 << 16) {
        frame.push(126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }
    frame.extend_from_slice(payload);
    frame
}

// --- SHA-1 + base64 implementations (no external deps) ---

fn sha1(message: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x6745_2301;
    let mut h1: u32 = 0xEFCD_AB89;
    let mut h2: u32 = 0x98BA_DCFE;
    let mut h3: u32 = 0x1032_5476;
    let mut h4: u32 = 0xC3D2_E1F0;

    let original_len = message.len();
    let bit_len: u64 = original_len as u64 * 8;
    let mut padded = message.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0_u32; 80];
        for (i, slot) in w.iter_mut().take(16).enumerate() {
            let offset = i * 4;
            *slot = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;
        for (i, word) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999_u32),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1_u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC_u32),
                _ => (b ^ c ^ d, 0xCA62_C1D6_u32),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut output = [0_u8; 20];
    output[0..4].copy_from_slice(&h0.to_be_bytes());
    output[4..8].copy_from_slice(&h1.to_be_bytes());
    output[8..12].copy_from_slice(&h2.to_be_bytes());
    output[12..16].copy_from_slice(&h3.to_be_bytes());
    output[16..20].copy_from_slice(&h4.to_be_bytes());
    output
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity((bytes.len() + 2) / 3 * 4);
    let mut index = 0;
    while index + 3 <= bytes.len() {
        let a = bytes[index];
        let b = bytes[index + 1];
        let c = bytes[index + 2];
        output.push(ALPHABET[(a >> 2) as usize] as char);
        output.push(ALPHABET[((a & 0x03) << 4 | b >> 4) as usize] as char);
        output.push(ALPHABET[((b & 0x0F) << 2 | c >> 6) as usize] as char);
        output.push(ALPHABET[(c & 0x3F) as usize] as char);
        index += 3;
    }
    let remainder = bytes.len() - index;
    if remainder == 1 {
        let a = bytes[index];
        output.push(ALPHABET[(a >> 2) as usize] as char);
        output.push(ALPHABET[((a & 0x03) << 4) as usize] as char);
        output.push('=');
        output.push('=');
    } else if remainder == 2 {
        let a = bytes[index];
        let b = bytes[index + 1];
        output.push(ALPHABET[(a >> 2) as usize] as char);
        output.push(ALPHABET[((a & 0x03) << 4 | b >> 4) as usize] as char);
        output.push(ALPHABET[((b & 0x0F) << 2) as usize] as char);
        output.push('=');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_request_extracts_path_query_headers() {
        let raw = b"GET /realtime/v1/websocket?apikey=secret&vsn=2.0.0 HTTP/1.1\r\n\
                    Host: example.com\r\n\
                    Upgrade: websocket\r\n\
                    Connection: Upgrade\r\n\
                    Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                    Sec-WebSocket-Version: 13\r\n\r\n";
        let request = UpgradeRequest::parse(raw).expect("parse");
        assert_eq!(request.path, "/realtime/v1/websocket");
        assert_eq!(request.query["apikey"], "secret");
        assert_eq!(request.query["vsn"], "2.0.0");
        assert_eq!(request.headers["upgrade"], "websocket");
    }

    #[test]
    fn handshake_response_uses_rfc6455_test_vector() {
        // RFC 6455 §1.3 example: Sec-WebSocket-Key dGhlIHNhbXBsZSBub25jZQ==
        // -> Sec-WebSocket-Accept s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
        let raw =
            b"GET / HTTP/1.1\r\nHost: x\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n";
        let request = UpgradeRequest::parse(raw).expect("parse");
        let accept = request.accept_key().expect("accept");
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn text_frame_round_trip_through_decoder() {
        let frame = encode_text_frame("hello");
        let (opcode, payload, consumed) = decode_frame(&frame).expect("decode").expect("ready");
        assert_eq!(opcode, 0x1);
        assert_eq!(payload, b"hello");
        assert_eq!(consumed, frame.len());
    }

    #[test]
    fn masked_client_frame_is_unmasked() {
        // Client frame "hi" with mask bytes 0x37,0xfa,0x21,0x3d.
        let frame = vec![0x81, 0x82, 0x37, 0xfa, 0x21, 0x3d, b'h' ^ 0x37, b'i' ^ 0xfa];
        let (opcode, payload, consumed) = decode_frame(&frame).expect("decode").expect("ready");
        assert_eq!(opcode, 0x1);
        assert_eq!(payload, b"hi");
        assert_eq!(consumed, frame.len());
    }

    #[test]
    fn close_frame_is_well_formed() {
        let frame = encode_close_frame(1000, "bye");
        assert_eq!(frame[0] & 0x0F, 0x8);
        let length = frame[1] as usize;
        assert_eq!(length, 5);
        assert_eq!(&frame[2..4], &1000_u16.to_be_bytes());
        assert_eq!(&frame[4..7], b"bye");
    }

    #[test]
    fn sha1_matches_rfc_test_vector() {
        // RFC 3174: "abc" -> "a9993e364706816aba3e25717850c26c9cd0d89d"
        let digest = sha1(b"abc");
        assert_eq!(
            hex_lower(&digest),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
    }

    fn hex_lower(bytes: &[u8]) -> String {
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            output.push_str(&format!("{byte:02x}"));
        }
        output
    }
}
