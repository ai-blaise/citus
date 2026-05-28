// FEATURE: T7

//! PostgreSQL v3 wire-protocol codec for the ai-blaise/citus pool.
//!
//! The pool needs typed parsing of the extended-query frames it intends to
//! buffer between `Sync` points and of the cancel-request envelopes it rewrites
//! to multiplex virtual PIDs. This crate is a self-contained Rust port of the
//! shape and semantics of `jackc/pgx` `pgproto3` (MIT), scoped to the messages
//! the pool actually inspects.
//!
//! Design constraints:
//!   1. No external dependencies. The pool's proxy hot path uses std-only
//!      `Vec<u8>` and `to_be_bytes`; the codec stays in the same shape so it
//!      composes without forcing the rest of the pool onto `bytes::BytesMut`.
//!   2. Round-trip equality: every `Encode` followed by a matching `Decode`
//!      MUST produce the original message. The test suite enforces this for
//!      each implemented message type.
//!   3. Bidirectional. Frontend (`Parse`, `Bind`, `Execute`, ...) and backend
//!      (`ErrorResponse`, `BackendKeyData`, `ReadyForQuery`, ...) messages
//!      share the same envelope reader/writer.
//!
//! Wire-frame envelope shapes (PostgreSQL v3):
//!
//! ```text
//!   regular frame:    [u8 tag] [u32 length] [body ...]
//!   startup envelope: [u32 length] [u32 code] [body ...]   (no tag byte)
//! ```
//!
//! `length` covers itself but NOT the tag byte. Startup envelopes carry
//! their code (protocol version / SSL / GSS / Cancel magic) where the tag
//! would otherwise live.

#![deny(missing_debug_implementations)]

pub mod auth;
pub mod codec;
pub mod envelope;
pub mod frontend;
pub mod backend;
pub mod startup;

// Auth + COPY-stream message types are forward-facing public surface: T7
// only counts them at the tag granularity, but T8 work (auth-state-aware
// pool routing, shard-aware COPY routing, COPY-stream-level observability)
// will consume the typed variants. Keep them re-exported.
pub use auth::{
    auth_codes, AuthFrontendFrame, AuthenticationRequest, GssResponseFrame, PasswordMessageFrame,
    SaslInitialResponseFrame, SaslResponseFrame,
};
pub use codec::{PgReader, PgWriteBuf, WireError};
pub use envelope::{FrameHeader, FRAME_HEADER_LEN};
pub use frontend::{
    BindFrame, CloseFrame, CloseTarget, CopyDataFrame, CopyDoneFrame, CopyFailFrame,
    DescribeFrame, DescribeTarget, ExecuteFrame, FlushFrame, FrontendMessage, ParseFrame,
    QueryFrame, SyncFrame, TerminateFrame,
};
// CopyBothResponse / CopyInResponse / CopyOutResponse + NegotiateProtocolVersion
// are forward-facing alongside the auth + COPY exports above. T7 today emits
// only frontend frame counters; backend-direction parsing will plug in here
// when T7's reverse-direction symmetric counter set graduates from alpha.
pub use backend::{
    BackendKeyDataFrame, BackendMessage, BindCompleteFrame, CloseCompleteFrame,
    CommandCompleteFrame, CopyBothResponseFrame, CopyInResponseFrame, CopyOutResponseFrame,
    DataRowFrame, EmptyQueryResponseFrame, ErrorField, ErrorResponseFrame,
    NegotiateProtocolVersionFrame, NoDataFrame, NoticeResponseFrame, NotificationResponseFrame,
    ParameterDescriptionFrame, ParameterStatusFrame, ParseCompleteFrame, PortalSuspendedFrame,
    ReadyForQueryFrame, ReadyTransactionStatus, RowDescriptionFrame, RowField,
};
pub use startup::{
    CancelRequest, GssEncRequest, SslRequest, StartupEnvelope, StartupMessage,
    CANCEL_REQUEST_CODE, GSSENC_REQUEST_CODE, PROTOCOL_VERSION_3_0, SSL_REQUEST_CODE,
};

/// Maximum number of bytes the codec will accept for any single message body.
/// This matches the upstream PostgreSQL limit of 1 GiB on `MaxAllocSize` and
/// caps memory exposure when a malformed peer claims a giant length.
pub const MAX_MESSAGE_BODY_BYTES: usize = 1 << 30;

/// Maximum number of bytes accepted in a startup envelope (matches the same
/// limit PostgreSQL applies to `MAX_STARTUP_PACKET_LENGTH`).
pub const MAX_STARTUP_ENVELOPE_BYTES: usize = 10_000;
