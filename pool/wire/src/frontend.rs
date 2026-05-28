// FEATURE: T7
// Derived from jackc/pgx pgproto3 (MIT).

//! Frontend message types (client -> server) for the PostgreSQL v3 wire
//! protocol. The pool buffers a subset of these between `Sync` points so
//! extended-query pipelines can be inspected, rewritten, and flushed as a
//! single batch.
//!
//! Tag bytes follow the upstream `pgproto3` shape:
//!   `P` Parse, `B` Bind, `D` Describe, `E` Execute, `S` Sync, `H` Flush,
//!   `C` Close, `Q` Query, `d` CopyData, `c` CopyDone, `f` CopyFail,
//!   `X` Terminate.

use crate::codec::{PgReader, PgWriteBuf, WireError};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FrontendMessage {
    Parse(ParseFrame),
    Bind(BindFrame),
    Describe(DescribeFrame),
    Execute(ExecuteFrame),
    Sync(SyncFrame),
    Flush(FlushFrame),
    Close(CloseFrame),
    Query(QueryFrame),
    CopyData(CopyDataFrame),
    CopyDone(CopyDoneFrame),
    CopyFail(CopyFailFrame),
    Terminate(TerminateFrame),
}

impl FrontendMessage {
    pub fn wire_tag(&self) -> u8 {
        match self {
            Self::Parse(_) => b'P',
            Self::Bind(_) => b'B',
            Self::Describe(_) => b'D',
            Self::Execute(_) => b'E',
            Self::Sync(_) => b'S',
            Self::Flush(_) => b'H',
            Self::Close(_) => b'C',
            Self::Query(_) => b'Q',
            Self::CopyData(_) => b'd',
            Self::CopyDone(_) => b'c',
            Self::CopyFail(_) => b'f',
            Self::Terminate(_) => b'X',
        }
    }

    pub fn encode(&self, buf: &mut PgWriteBuf) {
        match self {
            Self::Parse(frame) => frame.encode(buf),
            Self::Bind(frame) => frame.encode(buf),
            Self::Describe(frame) => frame.encode(buf),
            Self::Execute(frame) => frame.encode(buf),
            Self::Sync(frame) => frame.encode(buf),
            Self::Flush(frame) => frame.encode(buf),
            Self::Close(frame) => frame.encode(buf),
            Self::Query(frame) => frame.encode(buf),
            Self::CopyData(frame) => frame.encode(buf),
            Self::CopyDone(frame) => frame.encode(buf),
            Self::CopyFail(frame) => frame.encode(buf),
            Self::Terminate(frame) => frame.encode(buf),
        }
    }

    /// Decode one framed frontend message given its tag and body. The header
    /// is parsed by `envelope::FrameHeader` and dispatched here.
    pub fn decode(tag: u8, body: &[u8]) -> Result<Self, WireError> {
        Ok(match tag {
            b'P' => Self::Parse(ParseFrame::decode(body)?),
            b'B' => Self::Bind(BindFrame::decode(body)?),
            b'D' => Self::Describe(DescribeFrame::decode(body)?),
            b'E' => Self::Execute(ExecuteFrame::decode(body)?),
            b'S' => Self::Sync(SyncFrame::decode(body)?),
            b'H' => Self::Flush(FlushFrame::decode(body)?),
            b'C' => Self::Close(CloseFrame::decode(body)?),
            b'Q' => Self::Query(QueryFrame::decode(body)?),
            b'd' => Self::CopyData(CopyDataFrame::decode(body)?),
            b'c' => Self::CopyDone(CopyDoneFrame::decode(body)?),
            b'f' => Self::CopyFail(CopyFailFrame::decode(body)?),
            b'X' => Self::Terminate(TerminateFrame::decode(body)?),
            unknown => {
                return Err(WireError::UnexpectedTag {
                    wanted: 0,
                    got: unknown,
                })
            }
        })
    }
}

// --- Parse (P) -------------------------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParseFrame {
    pub statement_name: String,
    pub query: String,
    pub parameter_oids: Vec<i32>,
}

impl ParseFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'P', |body| {
            body.write_cstring_str(&self.statement_name);
            body.write_cstring_str(&self.query);
            body.write_i16(self.parameter_oids.len() as i16);
            for oid in &self.parameter_oids {
                body.write_i32(*oid);
            }
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let statement_name = reader.read_cstring_utf8("statement_name")?.to_string();
        let query = reader.read_cstring_utf8("query")?.to_string();
        let count = reader.read_i16()? as usize;
        let mut parameter_oids = Vec::with_capacity(count);
        for _ in 0..count {
            parameter_oids.push(reader.read_i32()?);
        }
        Ok(Self {
            statement_name,
            query,
            parameter_oids,
        })
    }
}

// --- Bind (B) --------------------------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BindFrame {
    pub portal_name: String,
    pub statement_name: String,
    pub parameter_format_codes: Vec<i16>,
    pub parameters: Vec<Option<Vec<u8>>>,
    pub result_format_codes: Vec<i16>,
}

impl BindFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'B', |body| {
            body.write_cstring_str(&self.portal_name);
            body.write_cstring_str(&self.statement_name);
            body.write_i16(self.parameter_format_codes.len() as i16);
            for code in &self.parameter_format_codes {
                body.write_i16(*code);
            }
            body.write_i16(self.parameters.len() as i16);
            for parameter in &self.parameters {
                match parameter {
                    Some(bytes) => {
                        body.write_i32(bytes.len() as i32);
                        body.write_bytes(bytes);
                    }
                    None => body.write_i32(-1),
                }
            }
            body.write_i16(self.result_format_codes.len() as i16);
            for code in &self.result_format_codes {
                body.write_i16(*code);
            }
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let portal_name = reader.read_cstring_utf8("portal_name")?.to_string();
        let statement_name = reader.read_cstring_utf8("statement_name")?.to_string();
        let parameter_format_count = reader.read_i16()? as usize;
        let mut parameter_format_codes = Vec::with_capacity(parameter_format_count);
        for _ in 0..parameter_format_count {
            parameter_format_codes.push(reader.read_i16()?);
        }
        let parameter_count = reader.read_i16()? as usize;
        let mut parameters = Vec::with_capacity(parameter_count);
        for _ in 0..parameter_count {
            let length = reader.read_i32()?;
            if length == -1 {
                parameters.push(None);
            } else if length >= 0 {
                let bytes = reader.read_slice(length as usize)?;
                parameters.push(Some(bytes.to_vec()));
            } else {
                return Err(WireError::InvalidLength {
                    field: "bind-parameter",
                    value: length,
                });
            }
        }
        let result_format_count = reader.read_i16()? as usize;
        let mut result_format_codes = Vec::with_capacity(result_format_count);
        for _ in 0..result_format_count {
            result_format_codes.push(reader.read_i16()?);
        }
        Ok(Self {
            portal_name,
            statement_name,
            parameter_format_codes,
            parameters,
            result_format_codes,
        })
    }
}

// --- Describe (D) ----------------------------------------------------------

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DescribeTarget {
    Portal,
    Statement,
}

impl DescribeTarget {
    fn tag(self) -> u8 {
        match self {
            Self::Portal => b'P',
            Self::Statement => b'S',
        }
    }

    fn from_tag(value: u8) -> Result<Self, WireError> {
        match value {
            b'P' => Ok(Self::Portal),
            b'S' => Ok(Self::Statement),
            other => Err(WireError::InvalidEnumValue {
                field: "describe-target",
                value: other,
            }),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DescribeFrame {
    pub target: DescribeTarget,
    pub name: String,
}

impl DescribeFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'D', |body| {
            body.write_u8(self.target.tag());
            body.write_cstring_str(&self.name);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let target = DescribeTarget::from_tag(reader.read_u8()?)?;
        let name = reader.read_cstring_utf8("describe-name")?.to_string();
        Ok(Self { target, name })
    }
}

// --- Execute (E) -----------------------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExecuteFrame {
    pub portal_name: String,
    pub max_rows: i32,
}

impl ExecuteFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'E', |body| {
            body.write_cstring_str(&self.portal_name);
            body.write_i32(self.max_rows);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let portal_name = reader.read_cstring_utf8("portal_name")?.to_string();
        let max_rows = reader.read_i32()?;
        Ok(Self {
            portal_name,
            max_rows,
        })
    }
}

// --- Sync (S) / Flush (H) --------------------------------------------------

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct SyncFrame;

impl SyncFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'S', |_| {});
    }

    pub fn decode(_body: &[u8]) -> Result<Self, WireError> {
        Ok(Self)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct FlushFrame;

impl FlushFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'H', |_| {});
    }

    pub fn decode(_body: &[u8]) -> Result<Self, WireError> {
        Ok(Self)
    }
}

// --- Close (C) -------------------------------------------------------------

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CloseTarget {
    Portal,
    Statement,
}

impl CloseTarget {
    fn tag(self) -> u8 {
        match self {
            Self::Portal => b'P',
            Self::Statement => b'S',
        }
    }

    fn from_tag(value: u8) -> Result<Self, WireError> {
        match value {
            b'P' => Ok(Self::Portal),
            b'S' => Ok(Self::Statement),
            other => Err(WireError::InvalidEnumValue {
                field: "close-target",
                value: other,
            }),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CloseFrame {
    pub target: CloseTarget,
    pub name: String,
}

impl CloseFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'C', |body| {
            body.write_u8(self.target.tag());
            body.write_cstring_str(&self.name);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let target = CloseTarget::from_tag(reader.read_u8()?)?;
        let name = reader.read_cstring_utf8("close-name")?.to_string();
        Ok(Self { target, name })
    }
}

// --- Query (Q) -------------------------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QueryFrame {
    pub query: String,
}

impl QueryFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'Q', |body| {
            body.write_cstring_str(&self.query);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let query = reader.read_cstring_utf8("query")?.to_string();
        Ok(Self { query })
    }
}

// --- CopyData (d) / CopyDone (c) / CopyFail (f) ----------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CopyDataFrame {
    pub data: Vec<u8>,
}

impl CopyDataFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'd', |body| {
            body.write_bytes(&self.data);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        Ok(Self {
            data: body.to_vec(),
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct CopyDoneFrame;

impl CopyDoneFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'c', |_| {});
    }

    pub fn decode(_body: &[u8]) -> Result<Self, WireError> {
        Ok(Self)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CopyFailFrame {
    pub message: String,
}

impl CopyFailFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'f', |body| {
            body.write_cstring_str(&self.message);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let message = reader.read_cstring_utf8("copy-fail-message")?.to_string();
        Ok(Self { message })
    }
}

// --- Terminate (X) ---------------------------------------------------------

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct TerminateFrame;

impl TerminateFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'X', |_| {});
    }

    pub fn decode(_body: &[u8]) -> Result<Self, WireError> {
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::read_frame;

    fn roundtrip(message: FrontendMessage) -> FrontendMessage {
        let mut buf = PgWriteBuf::new();
        message.encode(&mut buf);
        let bytes = buf.into_inner();
        let (header, body, consumed) = read_frame(&bytes).expect("frame");
        assert_eq!(consumed, bytes.len());
        FrontendMessage::decode(header.tag, body).expect("decode")
    }

    #[test]
    fn parse_roundtrip() {
        let original = FrontendMessage::Parse(ParseFrame {
            statement_name: "stmt-1".to_string(),
            query: "SELECT $1::int + $2::int".to_string(),
            parameter_oids: vec![23, 23],
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn bind_roundtrip_with_null_parameter() {
        let original = FrontendMessage::Bind(BindFrame {
            portal_name: String::new(),
            statement_name: "stmt-1".to_string(),
            parameter_format_codes: vec![1, 0],
            parameters: vec![Some(vec![0, 0, 0, 7]), None],
            result_format_codes: vec![0],
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn execute_with_max_rows() {
        let original = FrontendMessage::Execute(ExecuteFrame {
            portal_name: "p1".to_string(),
            max_rows: 1_000,
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn describe_portal_and_statement() {
        for target in [DescribeTarget::Portal, DescribeTarget::Statement] {
            let original = FrontendMessage::Describe(DescribeFrame {
                target,
                name: "x".to_string(),
            });
            assert_eq!(roundtrip(original.clone()), original);
        }
    }

    #[test]
    fn sync_flush_close_terminate_roundtrip() {
        for original in [
            FrontendMessage::Sync(SyncFrame),
            FrontendMessage::Flush(FlushFrame),
            FrontendMessage::Close(CloseFrame {
                target: CloseTarget::Portal,
                name: "p1".to_string(),
            }),
            FrontendMessage::Terminate(TerminateFrame),
        ] {
            assert_eq!(roundtrip(original.clone()), original);
        }
    }

    #[test]
    fn query_simple_roundtrip() {
        let original = FrontendMessage::Query(QueryFrame {
            query: "SELECT 1;".to_string(),
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn copy_data_copy_done_copy_fail_roundtrip() {
        for original in [
            FrontendMessage::CopyData(CopyDataFrame {
                data: b"row-data".to_vec(),
            }),
            FrontendMessage::CopyDone(CopyDoneFrame),
            FrontendMessage::CopyFail(CopyFailFrame {
                message: "client gave up".to_string(),
            }),
        ] {
            assert_eq!(roundtrip(original.clone()), original);
        }
    }
}
