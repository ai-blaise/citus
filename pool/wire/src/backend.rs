// FEATURE: T7
// Derived from jackc/pgx pgproto3 (MIT).

//! Backend message types (server -> client) for the PostgreSQL v3 wire
//! protocol. The pool produces a handful of these directly (`ErrorResponse`
//! during startup admission rejects) and inspects others when rewriting
//! `BackendKeyData` / forwarding `NotificationResponse` to subscribers.
//!
//! Tag bytes:
//!   `R` Authentication, `K` BackendKeyData, `2` BindComplete, `3` CloseComplete,
//!   `C` CommandComplete, `D` DataRow, `E` ErrorResponse, `I` EmptyQueryResponse,
//!   `n` NoData, `N` NoticeResponse, `A` NotificationResponse,
//!   `t` ParameterDescription, `S` ParameterStatus, `1` ParseComplete,
//!   `s` PortalSuspended, `Z` ReadyForQuery, `T` RowDescription,
//!   `G` CopyInResponse, `H` CopyOutResponse, `W` CopyBothResponse,
//!   `d` CopyData, `c` CopyDone.

use crate::codec::{PgReader, PgWriteBuf, WireError};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BackendMessage {
    BackendKeyData(BackendKeyDataFrame),
    BindComplete(BindCompleteFrame),
    CloseComplete(CloseCompleteFrame),
    CommandComplete(CommandCompleteFrame),
    CopyInResponse(CopyInResponseFrame),
    CopyOutResponse(CopyOutResponseFrame),
    CopyBothResponse(CopyBothResponseFrame),
    DataRow(DataRowFrame),
    EmptyQueryResponse(EmptyQueryResponseFrame),
    ErrorResponse(ErrorResponseFrame),
    NegotiateProtocolVersion(NegotiateProtocolVersionFrame),
    NoData(NoDataFrame),
    NoticeResponse(NoticeResponseFrame),
    NotificationResponse(NotificationResponseFrame),
    ParameterDescription(ParameterDescriptionFrame),
    ParameterStatus(ParameterStatusFrame),
    ParseComplete(ParseCompleteFrame),
    PortalSuspended(PortalSuspendedFrame),
    ReadyForQuery(ReadyForQueryFrame),
    RowDescription(RowDescriptionFrame),
}

impl BackendMessage {
    pub fn wire_tag(&self) -> u8 {
        match self {
            Self::BackendKeyData(_) => b'K',
            Self::BindComplete(_) => b'2',
            Self::CloseComplete(_) => b'3',
            Self::CommandComplete(_) => b'C',
            Self::CopyInResponse(_) => b'G',
            Self::CopyOutResponse(_) => b'H',
            Self::CopyBothResponse(_) => b'W',
            Self::DataRow(_) => b'D',
            Self::EmptyQueryResponse(_) => b'I',
            Self::ErrorResponse(_) => b'E',
            Self::NegotiateProtocolVersion(_) => b'v',
            Self::NoData(_) => b'n',
            Self::NoticeResponse(_) => b'N',
            Self::NotificationResponse(_) => b'A',
            Self::ParameterDescription(_) => b't',
            Self::ParameterStatus(_) => b'S',
            Self::ParseComplete(_) => b'1',
            Self::PortalSuspended(_) => b's',
            Self::ReadyForQuery(_) => b'Z',
            Self::RowDescription(_) => b'T',
        }
    }

    pub fn encode(&self, buf: &mut PgWriteBuf) {
        match self {
            Self::BackendKeyData(frame) => frame.encode(buf),
            Self::BindComplete(frame) => frame.encode(buf),
            Self::CloseComplete(frame) => frame.encode(buf),
            Self::CommandComplete(frame) => frame.encode(buf),
            Self::CopyInResponse(frame) => frame.encode(buf),
            Self::CopyOutResponse(frame) => frame.encode(buf),
            Self::CopyBothResponse(frame) => frame.encode(buf),
            Self::DataRow(frame) => frame.encode(buf),
            Self::EmptyQueryResponse(frame) => frame.encode(buf),
            Self::ErrorResponse(frame) => frame.encode(buf),
            Self::NegotiateProtocolVersion(frame) => frame.encode(buf),
            Self::NoData(frame) => frame.encode(buf),
            Self::NoticeResponse(frame) => frame.encode(buf),
            Self::NotificationResponse(frame) => frame.encode(buf),
            Self::ParameterDescription(frame) => frame.encode(buf),
            Self::ParameterStatus(frame) => frame.encode(buf),
            Self::ParseComplete(frame) => frame.encode(buf),
            Self::PortalSuspended(frame) => frame.encode(buf),
            Self::ReadyForQuery(frame) => frame.encode(buf),
            Self::RowDescription(frame) => frame.encode(buf),
        }
    }

    pub fn decode(tag: u8, body: &[u8]) -> Result<Self, WireError> {
        Ok(match tag {
            b'K' => Self::BackendKeyData(BackendKeyDataFrame::decode(body)?),
            b'2' => Self::BindComplete(BindCompleteFrame::decode(body)?),
            b'3' => Self::CloseComplete(CloseCompleteFrame::decode(body)?),
            b'C' => Self::CommandComplete(CommandCompleteFrame::decode(body)?),
            b'G' => Self::CopyInResponse(CopyInResponseFrame::decode(body)?),
            b'H' => Self::CopyOutResponse(CopyOutResponseFrame::decode(body)?),
            b'W' => Self::CopyBothResponse(CopyBothResponseFrame::decode(body)?),
            b'D' => Self::DataRow(DataRowFrame::decode(body)?),
            b'I' => Self::EmptyQueryResponse(EmptyQueryResponseFrame::decode(body)?),
            b'E' => Self::ErrorResponse(ErrorResponseFrame::decode(body)?),
            b'v' => Self::NegotiateProtocolVersion(NegotiateProtocolVersionFrame::decode(body)?),
            b'n' => Self::NoData(NoDataFrame::decode(body)?),
            b'N' => Self::NoticeResponse(NoticeResponseFrame::decode(body)?),
            b'A' => Self::NotificationResponse(NotificationResponseFrame::decode(body)?),
            b't' => Self::ParameterDescription(ParameterDescriptionFrame::decode(body)?),
            b'S' => Self::ParameterStatus(ParameterStatusFrame::decode(body)?),
            b'1' => Self::ParseComplete(ParseCompleteFrame::decode(body)?),
            b's' => Self::PortalSuspended(PortalSuspendedFrame::decode(body)?),
            b'Z' => Self::ReadyForQuery(ReadyForQueryFrame::decode(body)?),
            b'T' => Self::RowDescription(RowDescriptionFrame::decode(body)?),
            unknown => {
                return Err(WireError::UnexpectedTag {
                    wanted: 0,
                    got: unknown,
                })
            }
        })
    }
}

// --- BackendKeyData (K) ----------------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackendKeyDataFrame {
    pub process_id: i32,
    pub secret_key: i32,
}

impl BackendKeyDataFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'K', |body| {
            body.write_i32(self.process_id);
            body.write_i32(self.secret_key);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let process_id = reader.read_i32()?;
        let secret_key = reader.read_i32()?;
        Ok(Self {
            process_id,
            secret_key,
        })
    }
}

// --- BindComplete (2) / CloseComplete (3) / ParseComplete (1) --------------

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct BindCompleteFrame;
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct CloseCompleteFrame;
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct ParseCompleteFrame;

macro_rules! impl_empty_backend {
    ($name:ident, $tag:expr) => {
        impl $name {
            pub fn encode(&self, buf: &mut PgWriteBuf) {
                buf.write_tagged_frame($tag, |_| {});
            }

            pub fn decode(_body: &[u8]) -> Result<Self, WireError> {
                Ok(Self)
            }
        }
    };
}

impl_empty_backend!(BindCompleteFrame, b'2');
impl_empty_backend!(CloseCompleteFrame, b'3');
impl_empty_backend!(ParseCompleteFrame, b'1');

// --- EmptyQueryResponse (I) / NoData (n) / PortalSuspended (s) -------------

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct EmptyQueryResponseFrame;
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct NoDataFrame;
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct PortalSuspendedFrame;

impl_empty_backend!(EmptyQueryResponseFrame, b'I');
impl_empty_backend!(NoDataFrame, b'n');
impl_empty_backend!(PortalSuspendedFrame, b's');

// --- CommandComplete (C) ---------------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CommandCompleteFrame {
    pub tag: String,
}

impl CommandCompleteFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'C', |body| {
            body.write_cstring_str(&self.tag);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let tag = reader
            .read_cstring_utf8("command-complete-tag")?
            .to_string();
        Ok(Self { tag })
    }
}

// --- DataRow (D) -----------------------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DataRowFrame {
    pub columns: Vec<Option<Vec<u8>>>,
}

impl DataRowFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'D', |body| {
            body.write_i16(self.columns.len() as i16);
            for column in &self.columns {
                match column {
                    Some(bytes) => {
                        body.write_i32(bytes.len() as i32);
                        body.write_bytes(bytes);
                    }
                    None => body.write_i32(-1),
                }
            }
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let count = reader.read_i16()? as usize;
        let mut columns = Vec::with_capacity(count);
        for _ in 0..count {
            let length = reader.read_i32()?;
            if length == -1 {
                columns.push(None);
            } else if length >= 0 {
                let bytes = reader.read_slice(length as usize)?;
                columns.push(Some(bytes.to_vec()));
            } else {
                return Err(WireError::InvalidLength {
                    field: "data-row-column",
                    value: length,
                });
            }
        }
        Ok(Self { columns })
    }
}

// --- ErrorResponse (E) / NoticeResponse (N) --------------------------------

/// One field of an ErrorResponse / NoticeResponse.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ErrorField {
    pub tag: u8,
}

impl ErrorField {
    pub const SEVERITY: u8 = b'S';
    pub const SEVERITY_NONLOCALIZED: u8 = b'V';
    pub const CODE: u8 = b'C';
    pub const MESSAGE: u8 = b'M';
    pub const DETAIL: u8 = b'D';
    pub const HINT: u8 = b'H';
    pub const POSITION: u8 = b'P';
    pub const INTERNAL_POSITION: u8 = b'p';
    pub const INTERNAL_QUERY: u8 = b'q';
    pub const WHERE: u8 = b'W';
    pub const SCHEMA: u8 = b's';
    pub const TABLE: u8 = b't';
    pub const COLUMN: u8 = b'c';
    pub const DATA_TYPE: u8 = b'd';
    pub const CONSTRAINT: u8 = b'n';
    pub const FILE: u8 = b'F';
    pub const LINE: u8 = b'L';
    pub const ROUTINE: u8 = b'R';
}

fn encode_error_fields(buf: &mut PgWriteBuf, fields: &[(u8, String)]) {
    for (tag, value) in fields {
        buf.write_u8(*tag);
        buf.write_cstring_str(value);
    }
    buf.write_u8(0);
}

fn decode_error_fields(body: &[u8]) -> Result<Vec<(u8, String)>, WireError> {
    let mut reader = PgReader::new(body);
    let mut fields = Vec::new();
    loop {
        let tag = reader.read_u8()?;
        if tag == 0 {
            break;
        }
        let value = reader.read_cstring_utf8("error-field")?.to_string();
        fields.push((tag, value));
    }
    Ok(fields)
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ErrorResponseFrame {
    pub fields: Vec<(u8, String)>,
}

impl ErrorResponseFrame {
    /// Convenience constructor for the FATAL-class errors the pool emits
    /// during startup-admission rejection.
    pub fn fatal(sqlstate: &str, message: &str) -> Self {
        Self {
            fields: vec![
                (ErrorField::SEVERITY, "FATAL".to_string()),
                (ErrorField::SEVERITY_NONLOCALIZED, "FATAL".to_string()),
                (ErrorField::CODE, sanitize_field(sqlstate)),
                (ErrorField::MESSAGE, sanitize_field(message)),
            ],
        }
    }

    /// Find the first occurrence of a tagged field.
    pub fn field(&self, tag: u8) -> Option<&str> {
        self.fields
            .iter()
            .find(|(field_tag, _)| *field_tag == tag)
            .map(|(_, value)| value.as_str())
    }

    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'E', |body| {
            encode_error_fields(body, &self.fields);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        Ok(Self {
            fields: decode_error_fields(body)?,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NoticeResponseFrame {
    pub fields: Vec<(u8, String)>,
}

impl NoticeResponseFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'N', |body| {
            encode_error_fields(body, &self.fields);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        Ok(Self {
            fields: decode_error_fields(body)?,
        })
    }
}

/// PostgreSQL `ErrorResponse` field values cannot contain embedded NUL bytes;
/// substitute a space so the codec never produces a truncated cstring.
fn sanitize_field(value: &str) -> String {
    value.replace('\0', " ")
}

// --- NotificationResponse (A) ---------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NotificationResponseFrame {
    pub process_id: i32,
    pub channel: String,
    pub payload: String,
}

impl NotificationResponseFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'A', |body| {
            body.write_i32(self.process_id);
            body.write_cstring_str(&self.channel);
            body.write_cstring_str(&self.payload);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let process_id = reader.read_i32()?;
        let channel = reader
            .read_cstring_utf8("notification-channel")?
            .to_string();
        let payload = reader
            .read_cstring_utf8("notification-payload")?
            .to_string();
        Ok(Self {
            process_id,
            channel,
            payload,
        })
    }
}

// --- ParameterDescription (t) ----------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParameterDescriptionFrame {
    pub parameter_oids: Vec<i32>,
}

impl ParameterDescriptionFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b't', |body| {
            body.write_i16(self.parameter_oids.len() as i16);
            for oid in &self.parameter_oids {
                body.write_i32(*oid);
            }
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let count = reader.read_i16()? as usize;
        let mut parameter_oids = Vec::with_capacity(count);
        for _ in 0..count {
            parameter_oids.push(reader.read_i32()?);
        }
        Ok(Self { parameter_oids })
    }
}

// --- ParameterStatus (S) ---------------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParameterStatusFrame {
    pub name: String,
    pub value: String,
}

impl ParameterStatusFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'S', |body| {
            body.write_cstring_str(&self.name);
            body.write_cstring_str(&self.value);
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let name = reader
            .read_cstring_utf8("parameter-status-name")?
            .to_string();
        let value = reader
            .read_cstring_utf8("parameter-status-value")?
            .to_string();
        Ok(Self { name, value })
    }
}

// --- ReadyForQuery (Z) -----------------------------------------------------

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ReadyTransactionStatus {
    Idle,
    InTransaction,
    InFailedTransaction,
}

impl ReadyTransactionStatus {
    pub fn tag(self) -> u8 {
        match self {
            Self::Idle => b'I',
            Self::InTransaction => b'T',
            Self::InFailedTransaction => b'E',
        }
    }

    pub fn from_tag(value: u8) -> Result<Self, WireError> {
        match value {
            b'I' => Ok(Self::Idle),
            b'T' => Ok(Self::InTransaction),
            b'E' => Ok(Self::InFailedTransaction),
            other => Err(WireError::InvalidEnumValue {
                field: "ready-for-query-status",
                value: other,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ReadyForQueryFrame {
    pub status: ReadyTransactionStatus,
}

impl ReadyForQueryFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'Z', |body| {
            body.write_u8(self.status.tag());
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let status = ReadyTransactionStatus::from_tag(reader.read_u8()?)?;
        Ok(Self { status })
    }
}

// --- RowDescription (T) ----------------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RowField {
    pub name: String,
    pub table_oid: i32,
    pub column_attribute: i16,
    pub data_type_oid: i32,
    pub data_type_size: i16,
    pub type_modifier: i32,
    pub format_code: i16,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RowDescriptionFrame {
    pub fields: Vec<RowField>,
}

impl RowDescriptionFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'T', |body| {
            body.write_i16(self.fields.len() as i16);
            for field in &self.fields {
                body.write_cstring_str(&field.name);
                body.write_i32(field.table_oid);
                body.write_i16(field.column_attribute);
                body.write_i32(field.data_type_oid);
                body.write_i16(field.data_type_size);
                body.write_i32(field.type_modifier);
                body.write_i16(field.format_code);
            }
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let count = reader.read_i16()? as usize;
        let mut fields = Vec::with_capacity(count);
        for _ in 0..count {
            let name = reader.read_cstring_utf8("row-field-name")?.to_string();
            let table_oid = reader.read_i32()?;
            let column_attribute = reader.read_i16()?;
            let data_type_oid = reader.read_i32()?;
            let data_type_size = reader.read_i16()?;
            let type_modifier = reader.read_i32()?;
            let format_code = reader.read_i16()?;
            fields.push(RowField {
                name,
                table_oid,
                column_attribute,
                data_type_oid,
                data_type_size,
                type_modifier,
                format_code,
            });
        }
        Ok(Self { fields })
    }
}

// --- CopyInResponse (G) / CopyOutResponse (H) / CopyBothResponse (W) -------

fn encode_copy_response(buf: &mut PgWriteBuf, tag: u8, frame: &CopyResponseBody) {
    buf.write_tagged_frame(tag, |body| {
        body.write_u8(frame.overall_format);
        body.write_i16(frame.column_format_codes.len() as i16);
        for code in &frame.column_format_codes {
            body.write_i16(*code);
        }
    });
}

fn decode_copy_response(body: &[u8]) -> Result<CopyResponseBody, WireError> {
    let mut reader = PgReader::new(body);
    let overall_format = reader.read_u8()?;
    let count = reader.read_i16()? as usize;
    let mut column_format_codes = Vec::with_capacity(count);
    for _ in 0..count {
        column_format_codes.push(reader.read_i16()?);
    }
    Ok(CopyResponseBody {
        overall_format,
        column_format_codes,
    })
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct CopyResponseBody {
    overall_format: u8,
    column_format_codes: Vec<i16>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CopyInResponseFrame {
    pub overall_format: u8,
    pub column_format_codes: Vec<i16>,
}

impl CopyInResponseFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        encode_copy_response(
            buf,
            b'G',
            &CopyResponseBody {
                overall_format: self.overall_format,
                column_format_codes: self.column_format_codes.clone(),
            },
        );
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let parsed = decode_copy_response(body)?;
        Ok(Self {
            overall_format: parsed.overall_format,
            column_format_codes: parsed.column_format_codes,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CopyOutResponseFrame {
    pub overall_format: u8,
    pub column_format_codes: Vec<i16>,
}

impl CopyOutResponseFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        encode_copy_response(
            buf,
            b'H',
            &CopyResponseBody {
                overall_format: self.overall_format,
                column_format_codes: self.column_format_codes.clone(),
            },
        );
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let parsed = decode_copy_response(body)?;
        Ok(Self {
            overall_format: parsed.overall_format,
            column_format_codes: parsed.column_format_codes,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CopyBothResponseFrame {
    pub overall_format: u8,
    pub column_format_codes: Vec<i16>,
}

impl CopyBothResponseFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        encode_copy_response(
            buf,
            b'W',
            &CopyResponseBody {
                overall_format: self.overall_format,
                column_format_codes: self.column_format_codes.clone(),
            },
        );
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let parsed = decode_copy_response(body)?;
        Ok(Self {
            overall_format: parsed.overall_format,
            column_format_codes: parsed.column_format_codes,
        })
    }
}

// --- NegotiateProtocolVersion (v) ------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NegotiateProtocolVersionFrame {
    pub newest_protocol_version: i32,
    pub unrecognized_options: Vec<String>,
}

impl NegotiateProtocolVersionFrame {
    pub fn encode(&self, buf: &mut PgWriteBuf) {
        buf.write_tagged_frame(b'v', |body| {
            body.write_i32(self.newest_protocol_version);
            body.write_i32(self.unrecognized_options.len() as i32);
            for option in &self.unrecognized_options {
                body.write_cstring_str(option);
            }
        });
    }

    pub fn decode(body: &[u8]) -> Result<Self, WireError> {
        let mut reader = PgReader::new(body);
        let newest_protocol_version = reader.read_i32()?;
        let count = reader.read_i32()? as usize;
        let mut unrecognized_options = Vec::with_capacity(count);
        for _ in 0..count {
            let option = reader
                .read_cstring_utf8("negotiate-protocol-option")?
                .to_string();
            unrecognized_options.push(option);
        }
        Ok(Self {
            newest_protocol_version,
            unrecognized_options,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::read_frame;

    fn roundtrip(message: BackendMessage) -> BackendMessage {
        let mut buf = PgWriteBuf::new();
        message.encode(&mut buf);
        let bytes = buf.into_inner();
        let (header, body, consumed) = read_frame(&bytes).expect("frame");
        assert_eq!(consumed, bytes.len());
        BackendMessage::decode(header.tag, body).expect("decode")
    }

    #[test]
    fn backend_key_data_roundtrip() {
        let original = BackendMessage::BackendKeyData(BackendKeyDataFrame {
            process_id: 4242,
            secret_key: 0x4001,
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn error_response_fatal_helper() {
        let original = BackendMessage::ErrorResponse(ErrorResponseFrame::fatal(
            "28000",
            "auth introspection rejected token",
        ));
        let decoded = roundtrip(original.clone());
        assert_eq!(decoded, original);
        if let BackendMessage::ErrorResponse(frame) = decoded {
            assert_eq!(frame.field(ErrorField::CODE), Some("28000"));
            assert_eq!(
                frame.field(ErrorField::MESSAGE),
                Some("auth introspection rejected token")
            );
        } else {
            unreachable!()
        }
    }

    #[test]
    fn error_response_strips_embedded_nul() {
        let frame = ErrorResponseFrame::fatal("08006", "message with \0 nul");
        assert_eq!(frame.field(ErrorField::MESSAGE), Some("message with   nul"));
    }

    #[test]
    fn ready_for_query_status_variants() {
        for status in [
            ReadyTransactionStatus::Idle,
            ReadyTransactionStatus::InTransaction,
            ReadyTransactionStatus::InFailedTransaction,
        ] {
            let original = BackendMessage::ReadyForQuery(ReadyForQueryFrame { status });
            assert_eq!(roundtrip(original.clone()), original);
        }
    }

    #[test]
    fn data_row_with_null_column() {
        let original = BackendMessage::DataRow(DataRowFrame {
            columns: vec![Some(b"hello".to_vec()), None, Some(b"world".to_vec())],
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn row_description_full_field() {
        let original = BackendMessage::RowDescription(RowDescriptionFrame {
            fields: vec![RowField {
                name: "id".to_string(),
                table_oid: 16384,
                column_attribute: 1,
                data_type_oid: 23,
                data_type_size: 4,
                type_modifier: -1,
                format_code: 0,
            }],
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn parameter_status_roundtrip() {
        let original = BackendMessage::ParameterStatus(ParameterStatusFrame {
            name: "client_encoding".to_string(),
            value: "UTF8".to_string(),
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn notification_roundtrip() {
        let original = BackendMessage::NotificationResponse(NotificationResponseFrame {
            process_id: 100,
            channel: "events".to_string(),
            payload: "payload-1".to_string(),
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn empty_backend_frames_roundtrip() {
        for original in [
            BackendMessage::BindComplete(BindCompleteFrame),
            BackendMessage::CloseComplete(CloseCompleteFrame),
            BackendMessage::ParseComplete(ParseCompleteFrame),
            BackendMessage::EmptyQueryResponse(EmptyQueryResponseFrame),
            BackendMessage::NoData(NoDataFrame),
            BackendMessage::PortalSuspended(PortalSuspendedFrame),
        ] {
            assert_eq!(roundtrip(original.clone()), original);
        }
    }

    #[test]
    fn command_complete_roundtrip() {
        let original = BackendMessage::CommandComplete(CommandCompleteFrame {
            tag: "SELECT 1".to_string(),
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn copy_in_response_roundtrip() {
        let original = BackendMessage::CopyInResponse(CopyInResponseFrame {
            overall_format: 1,
            column_format_codes: vec![1, 0, 1],
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn copy_out_response_roundtrip() {
        let original = BackendMessage::CopyOutResponse(CopyOutResponseFrame {
            overall_format: 0,
            column_format_codes: vec![0; 4],
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn copy_both_response_roundtrip() {
        let original = BackendMessage::CopyBothResponse(CopyBothResponseFrame {
            overall_format: 1,
            column_format_codes: Vec::new(),
        });
        assert_eq!(roundtrip(original.clone()), original);
    }

    #[test]
    fn negotiate_protocol_version_roundtrip() {
        let original = BackendMessage::NegotiateProtocolVersion(NegotiateProtocolVersionFrame {
            newest_protocol_version: 0x00030000,
            unrecognized_options: vec!["extra_float_digits".to_string()],
        });
        assert_eq!(roundtrip(original.clone()), original);
    }
}
