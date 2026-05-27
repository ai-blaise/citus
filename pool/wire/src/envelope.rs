// FEATURE: T7

//! Frame envelope helpers shared by frontend + backend message paths.
//!
//! Every PostgreSQL v3 message after the startup envelope is framed as:
//!
//! ```text
//!     [u8 tag] [i32 length] [body ...]
//! ```
//!
//! where `length` covers itself and the body but NOT the tag byte.

use crate::codec::{PgReader, WireError};
use crate::MAX_MESSAGE_BODY_BYTES;

pub const FRAME_HEADER_LEN: usize = 5;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FrameHeader {
    pub tag: u8,
    pub body_length: usize,
}

impl FrameHeader {
    /// Parse a 5-byte header. The body length excludes the i32 itself.
    pub fn read(buf: &[u8]) -> Result<Self, WireError> {
        if buf.len() < FRAME_HEADER_LEN {
            return Err(WireError::Underflow {
                wanted: FRAME_HEADER_LEN,
                remaining: buf.len(),
            });
        }
        let tag = buf[0];
        let declared = i32::from_be_bytes(buf[1..5].try_into().unwrap());
        if declared < 4 {
            return Err(WireError::InvalidLength {
                field: "frame-length",
                value: declared,
            });
        }
        let body_length = declared as usize - 4;
        if body_length > MAX_MESSAGE_BODY_BYTES {
            return Err(WireError::MessageTooLarge {
                limit: MAX_MESSAGE_BODY_BYTES,
                declared: body_length,
            });
        }
        Ok(Self { tag, body_length })
    }

    /// Total frame size on the wire including the tag byte.
    pub fn total_frame_len(&self) -> usize {
        FRAME_HEADER_LEN + self.body_length
    }
}

/// Parse one complete framed message off the front of `buf`. Returns the
/// header, the body bytes, and the number of bytes consumed (header + body).
pub fn read_frame(buf: &[u8]) -> Result<(FrameHeader, &[u8], usize), WireError> {
    let header = FrameHeader::read(buf)?;
    let total = header.total_frame_len();
    if buf.len() < total {
        return Err(WireError::Underflow {
            wanted: total,
            remaining: buf.len(),
        });
    }
    let body = &buf[FRAME_HEADER_LEN..total];
    Ok((header, body, total))
}

/// Iterate as many complete framed messages as `buf` contains, returning the
/// total bytes consumed. The last partial frame, if any, is left untouched.
pub fn read_frames_until_short<F>(buf: &[u8], mut visitor: F) -> Result<usize, WireError>
where
    F: FnMut(FrameHeader, &[u8]) -> Result<(), WireError>,
{
    let mut cursor = 0;
    while cursor < buf.len() {
        let header = match FrameHeader::read(&buf[cursor..]) {
            Ok(header) => header,
            Err(WireError::Underflow { .. }) => break,
            Err(other) => return Err(other),
        };
        let total = header.total_frame_len();
        if buf.len() - cursor < total {
            break;
        }
        let body = &buf[cursor + FRAME_HEADER_LEN..cursor + total];
        visitor(header, body)?;
        cursor += total;
    }
    Ok(cursor)
}

/// Construct a `PgReader` for the body bytes referenced by a header.
pub fn body_reader<'a>(body: &'a [u8]) -> PgReader<'a> {
    PgReader::new(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_excludes_tag_from_length() {
        let mut frame = vec![b'1'];
        frame.extend_from_slice(&4_u32.to_be_bytes());
        let header = FrameHeader::read(&frame).expect("header");
        assert_eq!(header.tag, b'1');
        assert_eq!(header.body_length, 0);
        assert_eq!(header.total_frame_len(), 5);
    }

    #[test]
    fn short_header_underflows() {
        assert!(matches!(
            FrameHeader::read(&[b'1', 0, 0]),
            Err(WireError::Underflow {
                wanted: FRAME_HEADER_LEN,
                remaining: 3
            })
        ));
    }

    #[test]
    fn negative_length_rejected() {
        let mut frame = vec![b'1'];
        frame.extend_from_slice(&(-1i32).to_be_bytes());
        assert!(matches!(
            FrameHeader::read(&frame),
            Err(WireError::InvalidLength {
                field: "frame-length",
                value: -1
            })
        ));
    }

    #[test]
    fn iterator_stops_on_partial_frame() {
        let mut frame = vec![b'1'];
        frame.extend_from_slice(&4_u32.to_be_bytes());
        let mut buf = Vec::new();
        buf.extend_from_slice(&frame);
        buf.extend_from_slice(&frame);
        buf.extend_from_slice(&[b'2', 0, 0]);
        let mut count = 0;
        let consumed = read_frames_until_short(&buf, |_header, _body| {
            count += 1;
            Ok(())
        })
        .expect("iter");
        assert_eq!(count, 2);
        assert_eq!(consumed, 10);
    }
}
