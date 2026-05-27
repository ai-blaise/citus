// FEATURE: T7

//! Live extended-query pipeline smoke. Compiles the `pool/wire` codec into a
//! small binary that opens a raw TCP connection to a postgres backend, drives
//! `StartupMessage` -> `AuthenticationOk` -> `Parse / Bind / Execute / Sync`
//! using only the codec, and verifies the backend returns
//! `ParseComplete -> BindComplete -> DataRow -> CommandComplete ->
//! ReadyForQuery` in the order specified by the v3 wire spec.
//!
//! It also verifies the deterministic-failure semantics that motivate
//! explicit `Sync` boundaries: a pipeline containing one bad statement
//! followed by a good one must produce an `ErrorResponse` for the bad
//! statement, then advance to `ReadyForQuery` at the next `Sync` without
//! executing the queued frames that followed the failure.
//!
//! Usage:
//!   cargo run --example pipeline_live_smoke -p ai_blaise_citus_pool_wire \
//!       -- --host 127.0.0.1 --port 5432 --user postgres --database postgres

use ai_blaise_citus_pool_wire::{
    self as wire, BackendMessage, BindFrame, DescribeFrame, DescribeTarget, ErrorField,
    ExecuteFrame, FlushFrame, ParseFrame, PgWriteBuf, ReadyTransactionStatus, StartupEnvelope,
    StartupMessage, SyncFrame,
};
use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("usage error: {message}");
            return ExitCode::from(2);
        }
    };
    match run(&args) {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("pipeline_live_smoke FAILED: {error}");
            ExitCode::FAILURE
        }
    }
}

struct Args {
    host: String,
    port: u16,
    user: String,
    database: String,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut host = "127.0.0.1".to_string();
        let mut port: u16 = 5432;
        let mut user = "postgres".to_string();
        let mut database = "postgres".to_string();
        let mut iter = env::args().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--host" => host = iter.next().ok_or("missing value for --host")?,
                "--port" => {
                    port = iter
                        .next()
                        .ok_or("missing value for --port")?
                        .parse()
                        .map_err(|_| "invalid --port".to_string())?
                }
                "--user" => user = iter.next().ok_or("missing value for --user")?,
                "--database" => database = iter.next().ok_or("missing value for --database")?,
                other => return Err(format!("unknown argument {other}")),
            }
        }
        Ok(Self {
            host,
            port,
            user,
            database,
        })
    }
}

fn run(args: &Args) -> Result<String, String> {
    let mut stream = TcpStream::connect((args.host.as_str(), args.port))
        .map_err(|err| format!("connect {}:{}: {err}", args.host, args.port))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .map_err(map_io)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(15)))
        .map_err(map_io)?;

    let startup = StartupEnvelope::Startup(StartupMessage::new(vec![
        ("user".to_string(), args.user.clone()),
        ("database".to_string(), args.database.clone()),
    ]));
    let mut buf = PgWriteBuf::new();
    startup.encode(&mut buf);
    stream.write_all(buf.as_slice()).map_err(map_io)?;

    let mut session = Session::new(stream);
    session.expect_authentication_ok()?;
    session.drain_until_ready_for_query()?;

    // First pipeline: a good Parse/Bind/Execute/Sync. Should produce
    // ParseComplete -> BindComplete -> RowDescription? -> DataRow(s) ->
    // CommandComplete -> ReadyForQuery.
    let mut pipeline = PgWriteBuf::new();
    ParseFrame {
        statement_name: "good_stmt".to_string(),
        query: "SELECT $1::int4 + $2::int4 AS sum".to_string(),
        parameter_oids: vec![23, 23],
    }
    .encode(&mut pipeline);
    BindFrame {
        portal_name: String::new(),
        statement_name: "good_stmt".to_string(),
        parameter_format_codes: vec![0, 0],
        parameters: vec![Some(b"10".to_vec()), Some(b"32".to_vec())],
        result_format_codes: vec![0],
    }
    .encode(&mut pipeline);
    DescribeFrame {
        target: DescribeTarget::Portal,
        name: String::new(),
    }
    .encode(&mut pipeline);
    ExecuteFrame {
        portal_name: String::new(),
        max_rows: 0,
    }
    .encode(&mut pipeline);
    SyncFrame.encode(&mut pipeline);
    session.write_all(pipeline.as_slice())?;

    let mut sums: Vec<i64> = Vec::new();
    let mut parse_completes = 0_u32;
    let mut bind_completes = 0_u32;
    let mut command_completes = 0_u32;
    let mut ready_after_good: Option<ReadyTransactionStatus> = None;
    loop {
        let message = session.read_message()?;
        match message {
            BackendMessage::ParseComplete(_) => parse_completes += 1,
            BackendMessage::BindComplete(_) => bind_completes += 1,
            BackendMessage::DataRow(row) => {
                let column = row.columns.first().cloned().flatten().ok_or_else(|| {
                    "good DataRow missing first column".to_string()
                })?;
                let parsed = std::str::from_utf8(&column)
                    .map_err(|_| "DataRow column is not UTF-8".to_string())?
                    .parse::<i64>()
                    .map_err(|_| "DataRow column is not an integer".to_string())?;
                sums.push(parsed);
            }
            BackendMessage::CommandComplete(_) => command_completes += 1,
            BackendMessage::ReadyForQuery(ready) => {
                ready_after_good = Some(ready.status);
                break;
            }
            BackendMessage::NoticeResponse(_)
            | BackendMessage::RowDescription(_)
            | BackendMessage::ParameterDescription(_)
            | BackendMessage::NoData(_)
            | BackendMessage::EmptyQueryResponse(_) => {}
            BackendMessage::ErrorResponse(frame) => {
                return Err(format!(
                    "unexpected ErrorResponse during good pipeline: {:?}",
                    frame.field(ErrorField::MESSAGE)
                ));
            }
            other => return Err(format!("unexpected backend message {other:?}")),
        }
    }
    if parse_completes != 1 {
        return Err(format!("expected 1 ParseComplete, got {parse_completes}"));
    }
    if bind_completes != 1 {
        return Err(format!("expected 1 BindComplete, got {bind_completes}"));
    }
    if command_completes != 1 {
        return Err(format!("expected 1 CommandComplete, got {command_completes}"));
    }
    if sums != vec![42] {
        return Err(format!("expected DataRow sum=[42], got {sums:?}"));
    }
    let ready_status = ready_after_good.ok_or_else(|| "missing ReadyForQuery".to_string())?;
    if ready_status != ReadyTransactionStatus::Idle {
        return Err(format!(
            "expected ReadyForQuery=I after good pipeline, got {ready_status:?}"
        ));
    }

    // Second pipeline: deterministic failure semantics. A bad Parse must
    // produce an ErrorResponse and the subsequent good Execute MUST NOT
    // run before the next Sync (postgres queues an "error in failed
    // transaction" state in extended-query batches).
    let mut bad_pipeline = PgWriteBuf::new();
    ParseFrame {
        statement_name: "bad_stmt".to_string(),
        // Intentionally malformed SQL - postgres will emit ErrorResponse on Parse.
        query: "SELECT FROM WHERE WHERE".to_string(),
        parameter_oids: Vec::new(),
    }
    .encode(&mut bad_pipeline);
    BindFrame {
        portal_name: "bad_portal".to_string(),
        statement_name: "bad_stmt".to_string(),
        parameter_format_codes: Vec::new(),
        parameters: Vec::new(),
        result_format_codes: vec![0],
    }
    .encode(&mut bad_pipeline);
    ExecuteFrame {
        portal_name: "bad_portal".to_string(),
        max_rows: 0,
    }
    .encode(&mut bad_pipeline);
    FlushFrame.encode(&mut bad_pipeline);
    SyncFrame.encode(&mut bad_pipeline);
    session.write_all(bad_pipeline.as_slice())?;

    let mut error_observed = false;
    let mut bind_after_failure = 0_u32;
    let mut execute_after_failure = 0_u32;
    let mut ready_after_bad: Option<ReadyTransactionStatus> = None;
    loop {
        let message = session.read_message()?;
        match message {
            BackendMessage::ErrorResponse(_) => error_observed = true,
            BackendMessage::BindComplete(_) => bind_after_failure += 1,
            BackendMessage::CommandComplete(_) | BackendMessage::DataRow(_) => {
                execute_after_failure += 1
            }
            BackendMessage::ReadyForQuery(ready) => {
                ready_after_bad = Some(ready.status);
                break;
            }
            _ => {}
        }
    }
    if !error_observed {
        return Err("expected ErrorResponse for the malformed Parse, got none".to_string());
    }
    if bind_after_failure != 0 || execute_after_failure != 0 {
        return Err(format!(
            "frames after the failed Parse must not run before Sync; \
             bind_after_failure={bind_after_failure} execute_after_failure={execute_after_failure}"
        ));
    }
    let ready_status = ready_after_bad.ok_or_else(|| "missing ReadyForQuery after Sync".to_string())?;
    if ready_status != ReadyTransactionStatus::Idle {
        return Err(format!(
            "expected ReadyForQuery=I after Sync recovery, got {ready_status:?}"
        ));
    }

    Ok(format!(
        "pipeline_live_smoke\thost={}\tport={}\tuser={}\tdatabase={}\tgood_parse_complete=1\tgood_bind_complete=1\tgood_command_complete=1\tgood_sum=42\tbad_error_observed=true\tbad_bind_after_failure=0\tbad_execute_after_failure=0\tready_after_recovery=I",
        args.host, args.port, args.user, args.database
    ))
}

fn map_io(error: std::io::Error) -> String {
    format!("{error}")
}

struct Session {
    stream: TcpStream,
    rx_buf: Vec<u8>,
}

impl Session {
    fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            rx_buf: Vec::with_capacity(4096),
        }
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.stream.write_all(bytes).map_err(map_io)
    }

    fn fill_until(&mut self, needed: usize) -> Result<(), String> {
        while self.rx_buf.len() < needed {
            let mut tmp = [0_u8; 4096];
            let read = self.stream.read(&mut tmp).map_err(map_io)?;
            if read == 0 {
                return Err("backend closed the connection".to_string());
            }
            self.rx_buf.extend_from_slice(&tmp[..read]);
        }
        Ok(())
    }

    fn read_message(&mut self) -> Result<BackendMessage, String> {
        self.fill_until(wire::FRAME_HEADER_LEN)?;
        let header = wire::FrameHeader::read(&self.rx_buf).map_err(|err| err.to_string())?;
        let total = header.total_frame_len();
        self.fill_until(total)?;
        let body = self.rx_buf[wire::FRAME_HEADER_LEN..total].to_vec();
        self.rx_buf.drain(..total);
        BackendMessage::decode(header.tag, &body).map_err(|err| err.to_string())
    }

    fn expect_authentication_ok(&mut self) -> Result<(), String> {
        // Read frames until either AuthenticationOk (tag R, body starts with i32==0)
        // or an ErrorResponse. PostgreSQL with HOST_AUTH_METHOD=trust sends
        // exactly one Authentication message with auth-code 0 (AuthenticationOk).
        self.fill_until(wire::FRAME_HEADER_LEN)?;
        let header = wire::FrameHeader::read(&self.rx_buf).map_err(|err| err.to_string())?;
        if header.tag == b'E' {
            // Fall through to read the ErrorResponse for a useful message.
            let message = self.read_message()?;
            return Err(format!("backend rejected startup: {message:?}"));
        }
        if header.tag != b'R' {
            return Err(format!(
                "expected Authentication frame tag 'R', got 0x{:02x}",
                header.tag
            ));
        }
        let total = header.total_frame_len();
        self.fill_until(total)?;
        let body = &self.rx_buf[wire::FRAME_HEADER_LEN..total];
        if body.len() < 4 {
            return Err("AuthenticationOk body too short".to_string());
        }
        let auth_code = i32::from_be_bytes(body[0..4].try_into().unwrap());
        if auth_code != 0 {
            return Err(format!(
                "backend requested auth method {auth_code} but this smoke runs against HOST_AUTH_METHOD=trust"
            ));
        }
        self.rx_buf.drain(..total);
        Ok(())
    }

    fn drain_until_ready_for_query(&mut self) -> Result<(), String> {
        loop {
            match self.read_message()? {
                BackendMessage::ReadyForQuery(_) => return Ok(()),
                BackendMessage::ErrorResponse(frame) => {
                    return Err(format!(
                        "backend ErrorResponse during startup drain: {:?}",
                        frame.field(ErrorField::MESSAGE)
                    ))
                }
                _ => continue,
            }
        }
    }
}
