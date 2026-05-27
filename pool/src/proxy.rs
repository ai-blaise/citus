// FEATURE: O4
// FEATURE: O14
// FEATURE: Sec12
// FEATURE: Sec13
// FEATURE: T1
// FEATURE: T3
// FEATURE: T7

use crate::{
    admission::{PoolAdmissionConfig, PoolAdmissionController, PoolAdmissionError},
    auth_introspection::{PoolAuthConfig, PoolAuthError, PoolAuthGate},
    geoip::{route_report_for_client, ClosestReplicaTable, GeoIpError},
    runtime::{GeoRoutingPolicy, GeoRoutingRule, SessionSetting, SettingsBucketPolicy},
    settings_bucket::{SettingsBucketError, SettingsBucketPoolMap},
    trace_tap,
};
use std::error::Error;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

const DEFAULT_PROXY_LISTEN_ADDR: &str = "0.0.0.0:5432";
const DEFAULT_ADMIN_LISTEN_ADDR: &str = "0.0.0.0:8080";
const CONNECT_TIMEOUT: Duration = Duration::from_millis(750);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolProxyConfig {
    pub listen_addr: String,
    pub admin_addr: String,
    pub upstream_addr: String,
    pub client_cidr_allowlist: ClientCidrAllowlist,
    pub admission: PoolAdmissionConfig,
    pub auth: Option<PoolAuthConfig>,
    pub settings_bucket: Option<SettingsBucketPolicy>,
    pub geo_routing: Option<PoolGeoRoutingConfig>,
}

impl PoolProxyConfig {
    pub fn from_env() -> Result<Self, PoolProxyError> {
        let listen_addr = env_or_default("AI_BLAISE_POOL_LISTEN_ADDR", DEFAULT_PROXY_LISTEN_ADDR);
        let admin_addr = std::env::var("AI_BLAISE_POOL_ADMIN_ADDR")
            .or_else(|_| std::env::var("AI_BLAISE_LISTEN_ADDR"))
            .unwrap_or_else(|_| DEFAULT_ADMIN_LISTEN_ADDR.to_string());
        let upstream_addr = std::env::var("AI_BLAISE_POOL_UPSTREAM_ADDR")
            .map_err(|_| PoolProxyError::MissingEnv("AI_BLAISE_POOL_UPSTREAM_ADDR"))?;
        let client_cidr_allowlist = ClientCidrAllowlist::parse_csv(&env_or_default(
            "AI_BLAISE_POOL_CLIENT_CIDR_ALLOWLIST",
            "",
        ))?;
        let admission = PoolAdmissionConfig::from_env()?;
        let auth = PoolAuthConfig::from_env()?;
        let settings_bucket = settings_bucket_policy_from_env()?;
        let geo_routing = geo_routing_config_from_env()?;

        let config = Self {
            listen_addr,
            admin_addr,
            upstream_addr,
            client_cidr_allowlist,
            admission,
            auth,
            settings_bucket,
            geo_routing,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), PoolProxyError> {
        validate_addr("listen_addr", &self.listen_addr)?;
        validate_addr("admin_addr", &self.admin_addr)?;
        validate_addr("upstream_addr", &self.upstream_addr)?;
        if self.listen_addr == self.admin_addr {
            return Err(PoolProxyError::AddressCollision {
                left: "listen_addr",
                right: "admin_addr",
            });
        }
        self.admission.validate()?;
        if let Some(auth) = &self.auth {
            auth.validate()?;
        }
        if let Some(settings_bucket) = &self.settings_bucket {
            settings_bucket.validate().map_err(PoolProxyError::from)?;
        }
        if let Some(geo_routing) = &self.geo_routing {
            geo_routing.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolGeoRoutingConfig {
    pub policy: GeoRoutingPolicy,
    pub replicas: ClosestReplicaTable,
}

impl PoolGeoRoutingConfig {
    pub fn validate(&self) -> Result<(), PoolProxyError> {
        self.policy
            .validate()
            .map_err(|error| PoolProxyError::GeoRouting(error.to_string()))?;
        if self.replicas.region_count() == 0 {
            return Err(PoolProxyError::GeoRouting(
                "geo replica table must not be empty".to_string(),
            ));
        }
        Ok(())
    }

    pub fn route_upstream(
        &self,
        client_ip: IpAddr,
    ) -> Result<PoolGeoRouteDecision, PoolProxyError> {
        self.validate()?;
        let report = route_report_for_client(&self.policy, &self.replicas, client_ip, None)
            .map_err(PoolProxyError::from)?;
        let target = report.replica.target;
        Ok(PoolGeoRouteDecision {
            upstream_addr: format!("{}:{}", target.host, target.port),
            selected_region: report.selected_region,
            fallback_used: report.fallback_used,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PoolGeoRouteDecision {
    pub upstream_addr: String,
    pub selected_region: String,
    pub fallback_used: bool,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct ClientCidrAllowlist {
    entries: Vec<CidrBlock>,
}

impl ClientCidrAllowlist {
    pub fn parse_csv(value: &str) -> Result<Self, PoolProxyError> {
        let entries = value
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(CidrBlock::parse)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { entries })
    }

    pub fn allows(&self, ip: IpAddr) -> bool {
        self.entries.is_empty() || self.entries.iter().any(|entry| entry.contains(ip))
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn as_csv(&self) -> String {
        self.entries
            .iter()
            .map(|entry| entry.source.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct CidrBlock {
    network: IpAddr,
    prefix: u8,
    source: String,
}

impl CidrBlock {
    fn parse(value: &str) -> Result<Self, PoolProxyError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(PoolProxyError::InvalidCidr(value.to_string()));
        }

        let (ip_text, prefix) = if let Some((ip_text, prefix_text)) = trimmed.split_once('/') {
            let ip = ip_text
                .parse::<IpAddr>()
                .map_err(|_| PoolProxyError::InvalidCidr(trimmed.to_string()))?;
            let prefix = prefix_text
                .parse::<u8>()
                .map_err(|_| PoolProxyError::InvalidCidr(trimmed.to_string()))?;
            (ip, prefix)
        } else {
            let ip = trimmed
                .parse::<IpAddr>()
                .map_err(|_| PoolProxyError::InvalidCidr(trimmed.to_string()))?;
            let prefix = match ip {
                IpAddr::V4(_) => 32,
                IpAddr::V6(_) => 128,
            };
            (ip, prefix)
        };

        let max_prefix = match ip_text {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        if prefix > max_prefix {
            return Err(PoolProxyError::InvalidCidr(trimmed.to_string()));
        }

        Ok(Self {
            network: ip_text,
            prefix,
            source: trimmed.to_string(),
        })
    }

    fn contains(&self, candidate: IpAddr) -> bool {
        match (self.network, candidate) {
            (IpAddr::V4(network), IpAddr::V4(candidate)) => {
                cidr_contains_v4(network, candidate, self.prefix)
            }
            (IpAddr::V6(network), IpAddr::V6(candidate)) => {
                cidr_contains_v6(network, candidate, self.prefix)
            }
            _ => false,
        }
    }
}

fn cidr_contains_v4(network: Ipv4Addr, candidate: Ipv4Addr, prefix: u8) -> bool {
    let network = u32::from(network);
    let candidate = u32::from(candidate);
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    (network & mask) == (candidate & mask)
}

fn cidr_contains_v6(network: Ipv6Addr, candidate: Ipv6Addr, prefix: u8) -> bool {
    let network = u128::from(network);
    let candidate = u128::from(candidate);
    let mask = if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix)
    };
    (network & mask) == (candidate & mask)
}

#[derive(Debug)]
pub struct PoolProxyState {
    started_at: SystemTime,
    admission: PoolAdmissionController,
    auth: Option<PoolAuthGate>,
    active_connections: AtomicU64,
    accepted_connections: AtomicU64,
    completed_connections: AtomicU64,
    rejected_connections: AtomicU64,
    overloaded_connections: AtomicU64,
    tenant_quota_rejections: AtomicU64,
    auth_verified_connections: AtomicU64,
    auth_cache_hits: AtomicU64,
    auth_rejections: AtomicU64,
    settings_bucket: Option<Mutex<SettingsBucketPoolMap>>,
    settings_bucket_borrows: AtomicU64,
    settings_bucket_release_errors: AtomicU64,
    startup_timeouts: AtomicU64,
    fail_closed_routes: AtomicU64,
    upstream_connect_errors: AtomicU64,
    io_errors: AtomicU64,
    client_to_upstream_bytes: AtomicU64,
    upstream_to_client_bytes: AtomicU64,
    traceparent_tapped: AtomicU64,
    traceparent_absent: AtomicU64,
    geo_routes: AtomicU64,
    geo_fallback_routes: AtomicU64,
    // FEATURE: T7 — per-frame-type counters incremented as the client->upstream
    // forwarder decodes wire frames via pool/wire. Byte-transparency is
    // preserved: every frame is forwarded verbatim; the counters reflect what
    // crossed the proxy without rewriting the data path.
    ext_query_parse_frames: AtomicU64,
    ext_query_bind_frames: AtomicU64,
    ext_query_describe_frames: AtomicU64,
    ext_query_execute_frames: AtomicU64,
    ext_query_sync_frames: AtomicU64,
    ext_query_flush_frames: AtomicU64,
    ext_query_close_frames: AtomicU64,
    ext_query_simple_query_frames: AtomicU64,
    ext_query_copy_data_frames: AtomicU64,
    ext_query_terminate_frames: AtomicU64,
    ext_query_other_frames: AtomicU64,
    ext_query_decode_errors: AtomicU64,
}

impl PoolProxyState {
    pub fn new() -> Self {
        Self::with_config(PoolAdmissionConfig::default(), None)
            .expect("default pool admission config is valid")
    }

    pub fn with_admission_config(admission: PoolAdmissionConfig) -> Result<Self, PoolProxyError> {
        Self::with_config(admission, None)
    }

    pub fn with_config(
        admission: PoolAdmissionConfig,
        auth: Option<PoolAuthConfig>,
    ) -> Result<Self, PoolProxyError> {
        Self::with_proxy_config(admission, auth, None)
    }

    pub fn with_proxy_config(
        admission: PoolAdmissionConfig,
        auth: Option<PoolAuthConfig>,
        settings_bucket: Option<SettingsBucketPolicy>,
    ) -> Result<Self, PoolProxyError> {
        let auth = auth.map(PoolAuthGate::new).transpose()?;
        let settings_bucket = match settings_bucket {
            Some(policy) => Some(Mutex::new(SettingsBucketPoolMap::new(policy)?)),
            None => None,
        };
        Ok(Self {
            started_at: SystemTime::now(),
            admission: PoolAdmissionController::new(admission)?,
            auth,
            active_connections: AtomicU64::new(0),
            accepted_connections: AtomicU64::new(0),
            completed_connections: AtomicU64::new(0),
            rejected_connections: AtomicU64::new(0),
            overloaded_connections: AtomicU64::new(0),
            tenant_quota_rejections: AtomicU64::new(0),
            auth_verified_connections: AtomicU64::new(0),
            auth_cache_hits: AtomicU64::new(0),
            auth_rejections: AtomicU64::new(0),
            settings_bucket,
            settings_bucket_borrows: AtomicU64::new(0),
            settings_bucket_release_errors: AtomicU64::new(0),
            startup_timeouts: AtomicU64::new(0),
            fail_closed_routes: AtomicU64::new(0),
            upstream_connect_errors: AtomicU64::new(0),
            io_errors: AtomicU64::new(0),
            client_to_upstream_bytes: AtomicU64::new(0),
            upstream_to_client_bytes: AtomicU64::new(0),
            traceparent_tapped: AtomicU64::new(0),
            traceparent_absent: AtomicU64::new(0),
            geo_routes: AtomicU64::new(0),
            geo_fallback_routes: AtomicU64::new(0),
            ext_query_parse_frames: AtomicU64::new(0),
            ext_query_bind_frames: AtomicU64::new(0),
            ext_query_describe_frames: AtomicU64::new(0),
            ext_query_execute_frames: AtomicU64::new(0),
            ext_query_sync_frames: AtomicU64::new(0),
            ext_query_flush_frames: AtomicU64::new(0),
            ext_query_close_frames: AtomicU64::new(0),
            ext_query_simple_query_frames: AtomicU64::new(0),
            ext_query_copy_data_frames: AtomicU64::new(0),
            ext_query_terminate_frames: AtomicU64::new(0),
            ext_query_other_frames: AtomicU64::new(0),
            ext_query_decode_errors: AtomicU64::new(0),
        })
    }

    pub fn admission(&self) -> &PoolAdmissionController {
        &self.admission
    }

    pub fn auth_gate(&self) -> Option<&PoolAuthGate> {
        self.auth.as_ref()
    }

    pub fn active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }

    pub fn ext_query_counters(&self) -> ExtQueryCounters {
        ExtQueryCounters {
            parse: self.ext_query_parse_frames.load(Ordering::Relaxed),
            bind: self.ext_query_bind_frames.load(Ordering::Relaxed),
            describe: self.ext_query_describe_frames.load(Ordering::Relaxed),
            execute: self.ext_query_execute_frames.load(Ordering::Relaxed),
            sync: self.ext_query_sync_frames.load(Ordering::Relaxed),
            flush: self.ext_query_flush_frames.load(Ordering::Relaxed),
            close: self.ext_query_close_frames.load(Ordering::Relaxed),
            simple_query: self.ext_query_simple_query_frames.load(Ordering::Relaxed),
            copy_data: self.ext_query_copy_data_frames.load(Ordering::Relaxed),
            terminate: self.ext_query_terminate_frames.load(Ordering::Relaxed),
            other: self.ext_query_other_frames.load(Ordering::Relaxed),
            decode_errors: self.ext_query_decode_errors.load(Ordering::Relaxed),
        }
    }

    fn record_extended_frame(&self, tag: u8) {
        let counter = match tag {
            b'P' => &self.ext_query_parse_frames,
            b'B' => &self.ext_query_bind_frames,
            b'D' => &self.ext_query_describe_frames,
            b'E' => &self.ext_query_execute_frames,
            b'S' => &self.ext_query_sync_frames,
            b'H' => &self.ext_query_flush_frames,
            b'C' => &self.ext_query_close_frames,
            b'Q' => &self.ext_query_simple_query_frames,
            b'd' => &self.ext_query_copy_data_frames,
            b'X' => &self.ext_query_terminate_frames,
            _ => &self.ext_query_other_frames,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    fn record_decode_error(&self) {
        self.ext_query_decode_errors
            .fetch_add(1, Ordering::Relaxed);
    }

    fn acquire_settings_bucket(
        &self,
        tap: &trace_tap::StartupTraceTap,
    ) -> Result<Option<String>, PoolProxyError> {
        let Some(settings_bucket) = &self.settings_bucket else {
            return Ok(None);
        };
        let mut map = settings_bucket
            .lock()
            .map_err(|_| PoolProxyError::SettingsBucketLockPoisoned)?;
        let settings = tracked_settings_from_startup(tap, &map.policy().tracked_gucs);
        let entry = map.acquire(&settings)?;
        self.settings_bucket_borrows.fetch_add(1, Ordering::Relaxed);
        Ok(Some(entry.fingerprint))
    }

    fn release_settings_bucket(&self, fingerprint: &str) {
        let Some(settings_bucket) = &self.settings_bucket else {
            return;
        };
        let Ok(mut map) = settings_bucket.lock() else {
            self.settings_bucket_release_errors
                .fetch_add(1, Ordering::Relaxed);
            return;
        };
        if map.release(fingerprint).is_err() {
            self.settings_bucket_release_errors
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    fn settings_bucket_snapshot(&self) -> (usize, u32, u64, u64) {
        let borrowed = self.settings_bucket_borrows.load(Ordering::Relaxed);
        let release_errors = self.settings_bucket_release_errors.load(Ordering::Relaxed);
        let Some(settings_bucket) = &self.settings_bucket else {
            return (0, 0, borrowed, release_errors);
        };
        let Ok(map) = settings_bucket.lock() else {
            return (0, 0, borrowed, release_errors + 1);
        };
        (
            map.bucket_count(),
            map.total_assigned(),
            borrowed,
            release_errors,
        )
    }

    fn accepted(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        self.accepted_connections.fetch_add(1, Ordering::Relaxed);
    }

    fn completed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
        self.completed_connections.fetch_add(1, Ordering::Relaxed);
    }

    fn rejected(&self) {
        self.rejected_connections.fetch_add(1, Ordering::Relaxed);
    }

    fn overloaded(&self) {
        self.overloaded_connections.fetch_add(1, Ordering::Relaxed);
    }

    fn tenant_quota_rejected(&self) {
        self.tenant_quota_rejections.fetch_add(1, Ordering::Relaxed);
    }

    fn auth_verified(&self, cache_hit: bool) {
        self.auth_verified_connections
            .fetch_add(1, Ordering::Relaxed);
        if cache_hit {
            self.auth_cache_hits.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn auth_rejected(&self) {
        self.auth_rejections.fetch_add(1, Ordering::Relaxed);
    }

    fn startup_timeout(&self) {
        self.startup_timeouts.fetch_add(1, Ordering::Relaxed);
    }

    fn fail_closed_route(&self) {
        self.fail_closed_routes.fetch_add(1, Ordering::Relaxed);
    }

    fn connect_error(&self) {
        self.upstream_connect_errors.fetch_add(1, Ordering::Relaxed);
    }

    fn io_error(&self) {
        self.io_errors.fetch_add(1, Ordering::Relaxed);
    }

    fn add_client_bytes(&self, bytes: u64) {
        self.client_to_upstream_bytes
            .fetch_add(bytes, Ordering::Relaxed);
    }

    fn add_upstream_bytes(&self, bytes: u64) {
        self.upstream_to_client_bytes
            .fetch_add(bytes, Ordering::Relaxed);
    }

    fn traceparent_tapped(&self) {
        self.traceparent_tapped.fetch_add(1, Ordering::Relaxed);
    }

    fn traceparent_absent(&self) {
        self.traceparent_absent.fetch_add(1, Ordering::Relaxed);
    }

    fn geo_route(&self, fallback_used: bool) {
        self.geo_routes.fetch_add(1, Ordering::Relaxed);
        if fallback_used {
            self.geo_fallback_routes.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn traceparent_tapped_count(&self) -> u64 {
        self.traceparent_tapped.load(Ordering::Relaxed)
    }

    pub fn traceparent_absent_count(&self) -> u64 {
        self.traceparent_absent.load(Ordering::Relaxed)
    }

    fn uptime_seconds(&self) -> u64 {
        self.started_at
            .elapsed()
            .unwrap_or(Duration::ZERO)
            .as_secs()
    }
}

impl Default for PoolProxyState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AdminRequest {
    pub method: String,
    pub path: String,
}

impl AdminRequest {
    pub fn parse(bytes: &[u8]) -> Result<Self, PoolProxyError> {
        let request = std::str::from_utf8(bytes).map_err(|_| PoolProxyError::MalformedHttp)?;
        let request_line = request
            .lines()
            .next()
            .ok_or(PoolProxyError::MalformedHttp)?;
        let mut parts = request_line.split_whitespace();
        let method = parts.next().ok_or(PoolProxyError::MalformedHttp)?;
        let path = parts.next().ok_or(PoolProxyError::MalformedHttp)?;
        if !path.starts_with('/') {
            return Err(PoolProxyError::MalformedHttp);
        }
        Ok(Self {
            method: method.to_string(),
            path: path.to_string(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AdminResponse {
    pub status_code: u16,
    pub content_type: &'static str,
    pub body: String,
}

impl AdminResponse {
    fn to_http_string(&self) -> String {
        format!(
            "HTTP/1.1 {} {}\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            self.status_code,
            status_reason(self.status_code),
            self.content_type,
            self.body.len(),
            self.body,
        )
    }
}

pub fn run_pool_service_from_env() -> Result<(), PoolProxyError> {
    run_pool_service(PoolProxyConfig::from_env()?)
}

pub fn run_pool_service(config: PoolProxyConfig) -> Result<(), PoolProxyError> {
    config.validate()?;
    let proxy_listener = TcpListener::bind(&config.listen_addr)?;
    let admin_listener = TcpListener::bind(&config.admin_addr)?;
    let state = Arc::new(PoolProxyState::with_proxy_config(
        config.admission.clone(),
        config.auth.clone(),
        config.settings_bucket.clone(),
    )?);

    let admin_state = Arc::clone(&state);
    let admin_upstream = config.upstream_addr.clone();
    thread::spawn(move || {
        if let Err(error) = serve_admin(admin_listener, admin_state, admin_upstream) {
            eprintln!("pool admin server stopped: {error}");
        }
    });

    eprintln!(
        "ai-blaise pool proxy listening on {} and forwarding PostgreSQL traffic to {} with client CIDR allowlist {} max_active_connections {} auth_introspection {}",
        config.listen_addr,
        config.upstream_addr,
        if config.client_cidr_allowlist.is_empty() {
            "<allow-all>".to_string()
        } else {
            config.client_cidr_allowlist.as_csv()
        },
        config
            .admission
            .max_active_connections
            .map(|limit| limit.to_string())
            .unwrap_or_else(|| "<unbounded>".to_string()),
        config
            .auth
            .as_ref()
            .map(|auth| auth.introspection_url.as_str())
            .unwrap_or("<disabled>")
    );
    for client in proxy_listener.incoming() {
        let client = client?;
        let upstream_addr = config.upstream_addr.clone();
        let client_cidr_allowlist = config.client_cidr_allowlist.clone();
        let geo_routing = config.geo_routing.clone();
        let state = Arc::clone(&state);
        thread::spawn(move || {
            if let Err(error) = handle_proxy_connection(
                client,
                &upstream_addr,
                &client_cidr_allowlist,
                geo_routing.as_ref(),
                &state,
            ) {
                eprintln!("pool connection failed: {error}");
            }
        });
    }

    Ok(())
}

pub fn handle_proxy_connection(
    client: TcpStream,
    upstream_addr: &str,
    client_cidr_allowlist: &ClientCidrAllowlist,
    geo_routing: Option<&PoolGeoRoutingConfig>,
    state: &PoolProxyState,
) -> Result<(), PoolProxyError> {
    let client_ip = client.peer_addr()?.ip();
    if !client_cidr_allowlist.allows(client_ip) {
        state.rejected();
        return Err(PoolProxyError::ClientRejected {
            client_ip: client_ip.to_string(),
            allowlist: client_cidr_allowlist.as_csv(),
        });
    }

    let selected_upstream_addr = if let Some(geo_routing) = geo_routing {
        match geo_routing.route_upstream(client_ip) {
            Ok(decision) => {
                state.geo_route(decision.fallback_used);
                decision.upstream_addr
            }
            Err(error) => {
                state.rejected();
                state.fail_closed_route();
                return Err(error);
            }
        }
    } else {
        upstream_addr.to_string()
    };

    let permit = match state.admission().acquire_connection() {
        Ok(permit) => permit,
        Err(error) => {
            if matches!(error, PoolAdmissionError::Overloaded { .. }) {
                state.overloaded();
            }
            state.rejected();
            return Err(PoolProxyError::from(error));
        }
    };

    state.accepted();
    let result = proxy_connection(client, &selected_upstream_addr, state);
    state.completed();
    drop(permit);
    result
}

fn proxy_connection(
    client: TcpStream,
    upstream_addr: &str,
    state: &PoolProxyState,
) -> Result<(), PoolProxyError> {
    client.set_nodelay(true)?;

    // Read exactly one PostgreSQL startup envelope before opening an upstream
    // socket. That lets admission fail closed for slow clients, missing or
    // over-quota tenants, and unroutable upstreams without spending backend
    // capacity on traffic the pool will not forward.
    let startup_tap =
        match tap_startup_message_with_timeout(&client, state.admission().config().startup_timeout)
        {
            Ok(tap) => tap,
            Err(error) => {
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) {
                    state.startup_timeout();
                    state.rejected();
                    state.fail_closed_route();
                }
                state.io_error();
                return Err(PoolProxyError::from(error));
            }
        };

    if startup_tap.traceparent().is_some() {
        state.traceparent_tapped();
    } else {
        state.traceparent_absent();
    }
    eprintln!("ai-blaise pool {}", trace_tap::render_tap_log(&startup_tap));

    if let Err(error) = state.admission().admit_startup(&startup_tap) {
        state.tenant_quota_rejected();
        state.rejected();
        state.fail_closed_route();
        let _ = write_postgres_startup_error(&client, "53300", &error.to_string());
        return Err(PoolProxyError::from(error));
    }

    if let Some(auth) = state.auth_gate() {
        match auth.authorize_startup(&startup_tap) {
            Ok(decision) => {
                state.auth_verified(decision.cache_hit);
            }
            Err(error) => {
                state.auth_rejected();
                state.rejected();
                state.fail_closed_route();
                let _ = write_postgres_startup_error(&client, "28000", &error.to_string());
                return Err(PoolProxyError::from(error));
            }
        }
    }

    let settings_fingerprint = state.acquire_settings_bucket(&startup_tap)?;
    let prefix_bytes = startup_tap.sanitized_startup_bytes();
    let upstream = match connect_upstream(upstream_addr) {
        Ok(upstream) => upstream,
        Err(error) => {
            if let Some(fingerprint) = &settings_fingerprint {
                state.release_settings_bucket(fingerprint);
            }
            state.connect_error();
            state.fail_closed_route();
            let _ =
                write_postgres_startup_error(&client, "08006", "pool upstream is not reachable");
            return Err(error);
        }
    };
    upstream.set_nodelay(true)?;

    let result = thread::scope(|scope| {
        let mut client_reader = client.try_clone()?;
        let mut upstream_writer = upstream.try_clone()?;
        let prefix_bytes = prefix_bytes;
        let upload = scope.spawn(move || -> io::Result<u64> {
            upstream_writer.write_all(&prefix_bytes)?;
            let prefix_len = prefix_bytes.len() as u64;
            let copied = forward_client_to_upstream(
                &mut client_reader,
                &mut upstream_writer,
                state,
            )?;
            Ok(prefix_len + copied)
        });

        let mut upstream_reader = upstream;
        let mut client_writer = client;
        let downstream_result = copy_and_shutdown(&mut upstream_reader, &mut client_writer);
        let upload_result = upload.join().map_err(|_| PoolProxyError::WorkerPanicked)?;

        let mut first_error = None;

        match upload_result {
            Ok(bytes) => state.add_client_bytes(bytes),
            Err(error) => {
                state.io_error();
                first_error = Some(PoolProxyError::from(error));
            }
        }
        match downstream_result {
            Ok(bytes) => {
                state.add_upstream_bytes(bytes);
            }
            Err(error) => {
                state.io_error();
                if first_error.is_none() {
                    first_error = Some(PoolProxyError::from(error));
                }
            }
        }

        if let Some(error) = first_error {
            Err(error)
        } else {
            Ok(())
        }
    });

    if let Some(fingerprint) = settings_fingerprint {
        state.release_settings_bucket(&fingerprint);
    }

    result
}

fn copy_and_shutdown(reader: &mut TcpStream, writer: &mut TcpStream) -> io::Result<u64> {
    let bytes = io::copy(reader, writer)?;
    let _ = writer.shutdown(Shutdown::Write);
    Ok(bytes)
}

/// Snapshot of the per-pool wire-frame counters.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct ExtQueryCounters {
    pub parse: u64,
    pub bind: u64,
    pub describe: u64,
    pub execute: u64,
    pub sync: u64,
    pub flush: u64,
    pub close: u64,
    pub simple_query: u64,
    pub copy_data: u64,
    pub terminate: u64,
    pub other: u64,
    pub decode_errors: u64,
}

impl ExtQueryCounters {
    /// Total extended-query frames observed (P/B/D/E/S/H/C).
    pub fn extended_total(&self) -> u64 {
        self.parse + self.bind + self.describe + self.execute + self.sync + self.flush + self.close
    }
}

/// Tag bytes the PostgreSQL v3 wire protocol assigns to frontend frames the
/// pool currently understands. Anything outside this set - including the
/// start of a (second) StartupMessage after `SSLRequest`/`GSSENCRequest`, the
/// long-deprecated `F` (FunctionCall) fast-path that libpq no longer emits,
/// or any non-v3 traffic - falls back to byte-transparent copy so the
/// connection never stalls. The `p`-tag covers PasswordMessage and the three
/// SASL/GSS response frames; the pool counts them as a single tag class and
/// forwards them verbatim because their interpretation is context-dependent
/// on the most recent backend `R` (AuthenticationRequest) sub-code.
fn is_known_frontend_tag(tag: u8) -> bool {
    matches!(
        tag,
        b'P' | b'B' | b'D' | b'E' | b'S' | b'H' | b'C' | b'Q' | b'd' | b'c' | b'f' | b'X' | b'p'
    )
}

/// Client -> upstream forwarder that parses each PostgreSQL v3 wire frame via
/// `pool/wire`, accounts the frame tag into the matching `ExtQueryCounters`
/// field on `PoolProxyState`, and writes the bytes through to the upstream
/// verbatim. Byte-transparency for every tag is preserved; the codec is
/// observation-only on the hot path. The reverse direction
/// (upstream -> client) keeps the simpler `copy_and_shutdown` byte pump
/// because backend-frame accounting is not yet wired into the metrics
/// surface; adding it would symmetric-double the counter set and is tracked
/// under the alpha-deferred portion of the T7 contract.
///
/// Falls back to plain `io::copy` when the next byte is not a known v3
/// frontend tag, so SSL/GSS handshake replies, a second StartupMessage after
/// an `SSLRequest`/`GSSENCRequest`, or any non-v3 traffic flows through
/// unchanged without stalling.
fn forward_client_to_upstream(
    reader: &mut TcpStream,
    writer: &mut TcpStream,
    state: &PoolProxyState,
) -> io::Result<u64> {
    use ai_blaise_citus_pool_wire::{FrameHeader, WireError, FRAME_HEADER_LEN};

    let mut buffered: Vec<u8> = Vec::with_capacity(4096);
    let mut read_buf = [0u8; 4096];
    let mut total: u64 = 0;

    let fall_back_to_byte_copy = |buffered: &[u8],
                                  reader: &mut TcpStream,
                                  writer: &mut TcpStream,
                                  total: u64|
     -> io::Result<u64> {
        if !buffered.is_empty() {
            writer.write_all(buffered)?;
        }
        let copied_after = io::copy(reader, writer)?;
        let _ = writer.shutdown(Shutdown::Write);
        Ok(total + buffered.len() as u64 + copied_after)
    };

    loop {
        // Ensure we have a frame header.
        while buffered.len() < FRAME_HEADER_LEN {
            let read = reader.read(&mut read_buf)?;
            if read == 0 {
                if !buffered.is_empty() {
                    writer.write_all(&buffered)?;
                    total += buffered.len() as u64;
                }
                let _ = writer.shutdown(Shutdown::Write);
                return Ok(total);
            }
            buffered.extend_from_slice(&read_buf[..read]);
        }

        if !is_known_frontend_tag(buffered[0]) {
            state.record_decode_error();
            return fall_back_to_byte_copy(&buffered, reader, writer, total);
        }

        let header = match FrameHeader::read(&buffered) {
            Ok(header) => header,
            Err(WireError::InvalidLength { .. })
            | Err(WireError::MessageTooLarge { .. })
            | Err(WireError::Underflow { .. }) => {
                state.record_decode_error();
                return fall_back_to_byte_copy(&buffered, reader, writer, total);
            }
            Err(_) => {
                state.record_decode_error();
                return fall_back_to_byte_copy(&buffered, reader, writer, total);
            }
        };

        let total_frame_len = header.total_frame_len();
        while buffered.len() < total_frame_len {
            let read = reader.read(&mut read_buf)?;
            if read == 0 {
                writer.write_all(&buffered)?;
                total += buffered.len() as u64;
                let _ = writer.shutdown(Shutdown::Write);
                return Ok(total);
            }
            buffered.extend_from_slice(&read_buf[..read]);
        }

        state.record_extended_frame(header.tag);
        writer.write_all(&buffered[..total_frame_len])?;
        total += total_frame_len as u64;
        buffered.drain(..total_frame_len);
    }
}

fn tap_startup_message_with_timeout(
    client: &TcpStream,
    timeout: Duration,
) -> io::Result<trace_tap::StartupTraceTap> {
    client.set_read_timeout(Some(timeout))?;
    let mut startup_reader = client.try_clone()?;
    let result = trace_tap::tap_startup_message(&mut startup_reader);
    let reset = client.set_read_timeout(None);
    match (result, reset) {
        (Ok(tap), Ok(())) => Ok(tap),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

fn write_postgres_startup_error(
    mut client: &TcpStream,
    sqlstate: &str,
    message: &str,
) -> io::Result<()> {
    let mut buf = ai_blaise_citus_pool_wire::PgWriteBuf::new();
    ai_blaise_citus_pool_wire::ErrorResponseFrame::fatal(sqlstate, message).encode(&mut buf);
    client.write_all(buf.as_slice())?;
    let _ = client.shutdown(Shutdown::Write);
    Ok(())
}

fn serve_admin(
    listener: TcpListener,
    state: Arc<PoolProxyState>,
    upstream_addr: String,
) -> Result<(), PoolProxyError> {
    eprintln!(
        "ai-blaise pool admin server listening on {}",
        listener.local_addr()?
    );
    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut buffer = [0_u8; 8192];
        let read_len = stream.read(&mut buffer)?;
        let response = AdminRequest::parse(&buffer[..read_len])
            .map(|request| handle_admin_request(&request, &state, &upstream_addr))
            .unwrap_or_else(|error| AdminResponse {
                status_code: 400,
                content_type: "application/json",
                body: format!("{{\"error\":\"{}\"}}\n", escape_json(&error.to_string())),
            });
        stream.write_all(response.to_http_string().as_bytes())?;
    }
    Ok(())
}

pub fn handle_admin_request(
    request: &AdminRequest,
    state: &PoolProxyState,
    upstream_addr: &str,
) -> AdminResponse {
    if request.method != "GET" {
        return AdminResponse {
            status_code: 405,
            content_type: "application/json",
            body: "{\"error\":\"method not allowed\"}\n".to_string(),
        };
    }

    match request.path.as_str() {
        "/healthz" => AdminResponse {
            status_code: 200,
            content_type: "application/json",
            body: health_json(
                state,
                upstream_addr,
                true,
                connect_upstream(upstream_addr).is_ok(),
            ),
        },
        "/readyz" => {
            let upstream_ready = connect_upstream(upstream_addr).is_ok();
            AdminResponse {
                status_code: if upstream_ready { 200 } else { 503 },
                content_type: "application/json",
                body: health_json(state, upstream_addr, upstream_ready, upstream_ready),
            }
        }
        "/metrics" => AdminResponse {
            status_code: 200,
            content_type: "text/plain; version=0.0.4",
            body: metrics_text(state, upstream_addr),
        },
        _ => AdminResponse {
            status_code: 404,
            content_type: "application/json",
            body: "{\"error\":\"not found\"}\n".to_string(),
        },
    }
}

fn connect_upstream(upstream_addr: &str) -> Result<TcpStream, PoolProxyError> {
    let socket_addr = upstream_addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| PoolProxyError::InvalidAddress(upstream_addr.to_string()))?;
    TcpStream::connect_timeout(&socket_addr, CONNECT_TIMEOUT).map_err(PoolProxyError::from)
}

fn env_or_default(name: &str, default_value: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default_value.to_string())
}

fn geo_routing_config_from_env() -> Result<Option<PoolGeoRoutingConfig>, PoolProxyError> {
    let raw_replicas = env_or_default("AI_BLAISE_POOL_GEO_REPLICAS", "");
    if raw_replicas.trim().is_empty() {
        return Ok(None);
    }
    let default_region = std::env::var("AI_BLAISE_POOL_GEO_DEFAULT_REGION")
        .map_err(|_| PoolProxyError::MissingEnv("AI_BLAISE_POOL_GEO_DEFAULT_REGION"))?;
    let raw_rules = std::env::var("AI_BLAISE_POOL_GEO_RULES")
        .map_err(|_| PoolProxyError::MissingEnv("AI_BLAISE_POOL_GEO_RULES"))?;
    let rules = parse_geo_rules_env(&raw_rules)?;
    let replica_specs = raw_replicas
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if replica_specs.is_empty() {
        return Err(PoolProxyError::InvalidEnv {
            name: "AI_BLAISE_POOL_GEO_REPLICAS",
            value: raw_replicas,
            reason: "expected semicolon-separated region,latency_rank,host,port entries",
        });
    }
    let replica_refs = replica_specs.iter().map(String::as_str).collect::<Vec<_>>();
    let replicas = ClosestReplicaTable::from_specs(&replica_refs).map_err(PoolProxyError::from)?;
    let config = PoolGeoRoutingConfig {
        policy: GeoRoutingPolicy {
            default_region,
            rules,
        },
        replicas,
    };
    config.validate()?;
    Ok(Some(config))
}

fn parse_geo_rules_env(raw_rules: &str) -> Result<Vec<GeoRoutingRule>, PoolProxyError> {
    let mut rules = Vec::new();
    for entry in raw_rules
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let Some((cidr, region)) = entry.split_once('=') else {
            return Err(PoolProxyError::InvalidEnv {
                name: "AI_BLAISE_POOL_GEO_RULES",
                value: raw_rules.to_string(),
                reason: "expected semicolon-separated cidr=region entries",
            });
        };
        rules.push(GeoRoutingRule {
            cidr: cidr.trim().to_string(),
            region: region.trim().to_string(),
        });
    }
    if rules.is_empty() {
        return Err(PoolProxyError::InvalidEnv {
            name: "AI_BLAISE_POOL_GEO_RULES",
            value: raw_rules.to_string(),
            reason: "expected at least one cidr=region rule",
        });
    }
    Ok(rules)
}

fn settings_bucket_policy_from_env() -> Result<Option<SettingsBucketPolicy>, PoolProxyError> {
    let tracked_gucs = env_or_default("AI_BLAISE_POOL_SETTINGS_BUCKET_GUCS", "");
    let tracked_gucs = tracked_gucs
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if tracked_gucs.is_empty() {
        return Ok(None);
    }

    let max_connections = parse_u32_env("AI_BLAISE_POOL_SETTINGS_BUCKET_MAX_CONNECTIONS", 1024)?;
    let policy = SettingsBucketPolicy {
        bucket_name: env_or_default("AI_BLAISE_POOL_SETTINGS_BUCKET_NAME", "startup-gucs"),
        tracked_gucs,
        max_connections,
    };
    policy.validate().map_err(PoolProxyError::from)?;
    Ok(Some(policy))
}

fn parse_u32_env(name: &'static str, default_value: u32) -> Result<u32, PoolProxyError> {
    match std::env::var(name) {
        Ok(raw) => raw.parse::<u32>().map_err(|_| PoolProxyError::InvalidEnv {
            name,
            value: raw,
            reason: "expected unsigned integer",
        }),
        Err(_) => Ok(default_value),
    }
}

fn tracked_settings_from_startup(
    tap: &trace_tap::StartupTraceTap,
    tracked_gucs: &[String],
) -> Vec<SessionSetting> {
    tracked_gucs
        .iter()
        .filter_map(|name| {
            startup_setting_value(tap, name).map(|value| SessionSetting {
                name: name.clone(),
                value,
            })
        })
        .collect()
}

fn startup_setting_value(tap: &trace_tap::StartupTraceTap, name: &str) -> Option<String> {
    tap.startup_parameter(name)
        .map(ToOwned::to_owned)
        .or_else(|| {
            tap.startup_parameter("options")
                .and_then(|options| extract_options_assignment(options, name))
        })
}

fn extract_options_assignment(options: &str, key: &str) -> Option<String> {
    let tokens = options.split_whitespace().collect::<Vec<_>>();
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        if let Some(remainder) = token.strip_prefix("-c") {
            let assignment = if remainder.is_empty() {
                index += 1;
                if index >= tokens.len() {
                    break;
                }
                tokens[index]
            } else {
                remainder
            };
            if let Some((assignment_key, assignment_value)) = assignment.split_once('=') {
                if assignment_key.eq_ignore_ascii_case(key) {
                    return Some(assignment_value.to_string());
                }
            }
        }
        index += 1;
    }
    None
}

fn validate_addr(field: &'static str, address: &str) -> Result<(), PoolProxyError> {
    if address.trim().is_empty() {
        return Err(PoolProxyError::MissingAddress(field));
    }
    if !address.contains(':') {
        return Err(PoolProxyError::InvalidAddress(address.to_string()));
    }
    Ok(())
}

fn health_json(
    state: &PoolProxyState,
    upstream_addr: &str,
    ready: bool,
    upstream_ready: bool,
) -> String {
    let (
        settings_bucket_unique,
        settings_bucket_assigned,
        settings_bucket_borrows,
        settings_bucket_release_errors,
    ) = state.settings_bucket_snapshot();
    format!(
        r#"{{"component":"pool","ready":{},"upstream_ready":{},"upstream_addr":"{}","active_connections":{},"accepted_connections":{},"completed_connections":{},"rejected_connections":{},"overloaded_connections":{},"tenant_quota_rejections":{},"auth_verified_connections":{},"auth_cache_hits":{},"auth_rejections":{},"geo_routes":{},"geo_fallback_routes":{},"settings_bucket_unique_fingerprints":{},"settings_bucket_assigned_connections":{},"settings_bucket_backend_borrows":{},"settings_bucket_release_errors":{},"startup_timeouts":{},"fail_closed_routes":{},"upstream_connect_errors":{},"io_errors":{},"uptime_seconds":{}}}
"#,
        ready,
        upstream_ready,
        escape_json(upstream_addr),
        state.active_connections.load(Ordering::Relaxed),
        state.accepted_connections.load(Ordering::Relaxed),
        state.completed_connections.load(Ordering::Relaxed),
        state.rejected_connections.load(Ordering::Relaxed),
        state.overloaded_connections.load(Ordering::Relaxed),
        state.tenant_quota_rejections.load(Ordering::Relaxed),
        state.auth_verified_connections.load(Ordering::Relaxed),
        state.auth_cache_hits.load(Ordering::Relaxed),
        state.auth_rejections.load(Ordering::Relaxed),
        state.geo_routes.load(Ordering::Relaxed),
        state.geo_fallback_routes.load(Ordering::Relaxed),
        settings_bucket_unique,
        settings_bucket_assigned,
        settings_bucket_borrows,
        settings_bucket_release_errors,
        state.startup_timeouts.load(Ordering::Relaxed),
        state.fail_closed_routes.load(Ordering::Relaxed),
        state.upstream_connect_errors.load(Ordering::Relaxed),
        state.io_errors.load(Ordering::Relaxed),
        state.uptime_seconds(),
    )
}

fn metrics_text(state: &PoolProxyState, upstream_addr: &str) -> String {
    let upstream_ready = u8::from(connect_upstream(upstream_addr).is_ok());
    let (
        settings_bucket_unique,
        settings_bucket_assigned,
        settings_bucket_borrows,
        settings_bucket_release_errors,
    ) = state.settings_bucket_snapshot();
    format!(
        r#"# HELP ai_blaise_citus_pool_upstream_ready Whether the configured PostgreSQL upstream accepts TCP connections.
# TYPE ai_blaise_citus_pool_upstream_ready gauge
ai_blaise_citus_pool_upstream_ready{{upstream="{}"}} {}
# HELP ai_blaise_citus_pool_active_connections Active proxied PostgreSQL connections.
# TYPE ai_blaise_citus_pool_active_connections gauge
ai_blaise_citus_pool_active_connections {}
# HELP ai_blaise_citus_pool_requests_total Accepted PostgreSQL client connections.
# TYPE ai_blaise_citus_pool_requests_total counter
ai_blaise_citus_pool_requests_total {}
# HELP ai_blaise_citus_pool_rejected_connections_total PostgreSQL client connections rejected by CIDR allowlist, overload, startup timeout, or quota.
# TYPE ai_blaise_citus_pool_rejected_connections_total counter
ai_blaise_citus_pool_rejected_connections_total {}
# HELP ai_blaise_citus_pool_overloaded_connections_total PostgreSQL client connections rejected because active connection admission was full.
# TYPE ai_blaise_citus_pool_overloaded_connections_total counter
ai_blaise_citus_pool_overloaded_connections_total {}
# HELP ai_blaise_citus_pool_tenant_quota_rejections_total PostgreSQL client connections rejected by tenant quota admission.
# TYPE ai_blaise_citus_pool_tenant_quota_rejections_total counter
ai_blaise_citus_pool_tenant_quota_rejections_total {}
# HELP ai_blaise_citus_pool_auth_verified_connections_total PostgreSQL client connections admitted by auth introspection.
# TYPE ai_blaise_citus_pool_auth_verified_connections_total counter
ai_blaise_citus_pool_auth_verified_connections_total {}
# HELP ai_blaise_citus_pool_auth_cache_hits_total PostgreSQL client connections admitted from the auth introspection cache.
# TYPE ai_blaise_citus_pool_auth_cache_hits_total counter
ai_blaise_citus_pool_auth_cache_hits_total {}
# HELP ai_blaise_citus_pool_auth_rejections_total PostgreSQL client connections rejected by auth introspection.
# TYPE ai_blaise_citus_pool_auth_rejections_total counter
ai_blaise_citus_pool_auth_rejections_total {}
# HELP ai_blaise_citus_pool_geo_routes_total Connections routed through the GeoIP replica table.
# TYPE ai_blaise_citus_pool_geo_routes_total counter
ai_blaise_citus_pool_geo_routes_total {}
# HELP ai_blaise_citus_pool_geo_fallback_routes_total GeoIP-routed connections that used the default-region fallback.
# TYPE ai_blaise_citus_pool_geo_fallback_routes_total counter
ai_blaise_citus_pool_geo_fallback_routes_total {}
# HELP ai_blaise_citus_pool_settings_bucket_unique_fingerprints Unique tracked-GUC fingerprints observed by the pool.
# TYPE ai_blaise_citus_pool_settings_bucket_unique_fingerprints gauge
ai_blaise_citus_pool_settings_bucket_unique_fingerprints {}
# HELP ai_blaise_citus_pool_settings_bucket_assigned_connections Active client connections assigned to settings buckets.
# TYPE ai_blaise_citus_pool_settings_bucket_assigned_connections gauge
ai_blaise_citus_pool_settings_bucket_assigned_connections {}
# HELP ai_blaise_citus_pool_settings_bucket_backend_borrows_total Accepted backend assignments recorded by settings-bucket accounting.
# TYPE ai_blaise_citus_pool_settings_bucket_backend_borrows_total counter
ai_blaise_citus_pool_settings_bucket_backend_borrows_total {}
# HELP ai_blaise_citus_pool_settings_bucket_release_errors_total Settings-bucket release accounting errors.
# TYPE ai_blaise_citus_pool_settings_bucket_release_errors_total counter
ai_blaise_citus_pool_settings_bucket_release_errors_total {}
# HELP ai_blaise_citus_pool_startup_timeouts_total Client connections closed before a complete startup envelope arrived.
# TYPE ai_blaise_citus_pool_startup_timeouts_total counter
ai_blaise_citus_pool_startup_timeouts_total {}
# HELP ai_blaise_citus_pool_fail_closed_routes_total Connections intentionally routed nowhere because admission or upstream safety checks failed.
# TYPE ai_blaise_citus_pool_fail_closed_routes_total counter
ai_blaise_citus_pool_fail_closed_routes_total {}
# HELP ai_blaise_citus_pool_errors_total Upstream connect and proxy I/O errors.
# TYPE ai_blaise_citus_pool_errors_total counter
ai_blaise_citus_pool_errors_total {}
# HELP ai_blaise_citus_pool_client_to_upstream_bytes_total Bytes copied from clients to upstream PostgreSQL.
# TYPE ai_blaise_citus_pool_client_to_upstream_bytes_total counter
ai_blaise_citus_pool_client_to_upstream_bytes_total {}
# HELP ai_blaise_citus_pool_upstream_to_client_bytes_total Bytes copied from upstream PostgreSQL to clients.
# TYPE ai_blaise_citus_pool_upstream_to_client_bytes_total counter
ai_blaise_citus_pool_upstream_to_client_bytes_total {}
# HELP ai_blaise_citus_pool_traceparent_tapped_total Client connections whose startup envelope carried a W3C traceparent.
# TYPE ai_blaise_citus_pool_traceparent_tapped_total counter
ai_blaise_citus_pool_traceparent_tapped_total {}
# HELP ai_blaise_citus_pool_traceparent_absent_total Client connections whose startup envelope did not carry a W3C traceparent.
# TYPE ai_blaise_citus_pool_traceparent_absent_total counter
ai_blaise_citus_pool_traceparent_absent_total {}
# HELP ai_blaise_citus_pool_ext_query_frames_total PostgreSQL v3 wire frames observed by the pool/wire codec on the client->upstream path, by tag.
# TYPE ai_blaise_citus_pool_ext_query_frames_total counter
ai_blaise_citus_pool_ext_query_frames_total{{frame="Parse"}} {}
ai_blaise_citus_pool_ext_query_frames_total{{frame="Bind"}} {}
ai_blaise_citus_pool_ext_query_frames_total{{frame="Describe"}} {}
ai_blaise_citus_pool_ext_query_frames_total{{frame="Execute"}} {}
ai_blaise_citus_pool_ext_query_frames_total{{frame="Sync"}} {}
ai_blaise_citus_pool_ext_query_frames_total{{frame="Flush"}} {}
ai_blaise_citus_pool_ext_query_frames_total{{frame="Close"}} {}
ai_blaise_citus_pool_ext_query_frames_total{{frame="Query"}} {}
ai_blaise_citus_pool_ext_query_frames_total{{frame="CopyData"}} {}
ai_blaise_citus_pool_ext_query_frames_total{{frame="Terminate"}} {}
ai_blaise_citus_pool_ext_query_frames_total{{frame="Other"}} {}
# HELP ai_blaise_citus_pool_ext_query_decode_errors_total Wire-frame decode errors that fell back to byte-transparent forwarding.
# TYPE ai_blaise_citus_pool_ext_query_decode_errors_total counter
ai_blaise_citus_pool_ext_query_decode_errors_total {}
"#,
        escape_prometheus_label(upstream_addr),
        upstream_ready,
        state.active_connections.load(Ordering::Relaxed),
        state.accepted_connections.load(Ordering::Relaxed),
        state.rejected_connections.load(Ordering::Relaxed),
        state.overloaded_connections.load(Ordering::Relaxed),
        state.tenant_quota_rejections.load(Ordering::Relaxed),
        state.auth_verified_connections.load(Ordering::Relaxed),
        state.auth_cache_hits.load(Ordering::Relaxed),
        state.auth_rejections.load(Ordering::Relaxed),
        state.geo_routes.load(Ordering::Relaxed),
        state.geo_fallback_routes.load(Ordering::Relaxed),
        settings_bucket_unique,
        settings_bucket_assigned,
        settings_bucket_borrows,
        settings_bucket_release_errors,
        state.startup_timeouts.load(Ordering::Relaxed),
        state.fail_closed_routes.load(Ordering::Relaxed),
        state.upstream_connect_errors.load(Ordering::Relaxed)
            + state.io_errors.load(Ordering::Relaxed),
        state.client_to_upstream_bytes.load(Ordering::Relaxed),
        state.upstream_to_client_bytes.load(Ordering::Relaxed),
        state.traceparent_tapped.load(Ordering::Relaxed),
        state.traceparent_absent.load(Ordering::Relaxed),
        state.ext_query_parse_frames.load(Ordering::Relaxed),
        state.ext_query_bind_frames.load(Ordering::Relaxed),
        state.ext_query_describe_frames.load(Ordering::Relaxed),
        state.ext_query_execute_frames.load(Ordering::Relaxed),
        state.ext_query_sync_frames.load(Ordering::Relaxed),
        state.ext_query_flush_frames.load(Ordering::Relaxed),
        state.ext_query_close_frames.load(Ordering::Relaxed),
        state.ext_query_simple_query_frames.load(Ordering::Relaxed),
        state.ext_query_copy_data_frames.load(Ordering::Relaxed),
        state.ext_query_terminate_frames.load(Ordering::Relaxed),
        state.ext_query_other_frames.load(Ordering::Relaxed),
        state.ext_query_decode_errors.load(Ordering::Relaxed),
    )
}

fn status_reason(status_code: u16) -> &'static str {
    match status_code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn escape_prometheus_label(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[derive(Debug, Eq, PartialEq)]
pub enum PoolProxyError {
    Admission(PoolAdmissionError),
    Auth(PoolAuthError),
    AddressCollision {
        left: &'static str,
        right: &'static str,
    },
    ClientRejected {
        client_ip: String,
        allowlist: String,
    },
    InvalidAddress(String),
    InvalidCidr(String),
    Io(String),
    MalformedHttp,
    MissingAddress(&'static str),
    MissingEnv(&'static str),
    InvalidEnv {
        name: &'static str,
        value: String,
        reason: &'static str,
    },
    SettingsBucket(SettingsBucketError),
    GeoRouting(String),
    SettingsBucketLockPoisoned,
    WorkerPanicked,
}

impl fmt::Display for PoolProxyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admission(error) => write!(formatter, "{error}"),
            Self::Auth(error) => write!(formatter, "{error}"),
            Self::AddressCollision { left, right } => {
                write!(formatter, "{left} must differ from {right}")
            }
            Self::ClientRejected {
                client_ip,
                allowlist,
            } => {
                write!(
                    formatter,
                    "client {client_ip} is outside pool CIDR allowlist {allowlist}"
                )
            }
            Self::InvalidAddress(address) => write!(formatter, "invalid address: {address}"),
            Self::InvalidCidr(cidr) => write!(formatter, "invalid CIDR allowlist entry: {cidr}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::MalformedHttp => write!(formatter, "malformed HTTP request"),
            Self::MissingAddress(field) => write!(formatter, "{field} must not be empty"),
            Self::MissingEnv(name) => write!(formatter, "{name} is required"),
            Self::InvalidEnv {
                name,
                value,
                reason,
            } => {
                write!(formatter, "{name}={value:?} is invalid: {reason}")
            }
            Self::SettingsBucket(error) => write!(formatter, "{error}"),
            Self::GeoRouting(error) => write!(formatter, "geo routing error: {error}"),
            Self::SettingsBucketLockPoisoned => write!(formatter, "settings bucket lock poisoned"),
            Self::WorkerPanicked => write!(formatter, "proxy worker panicked"),
        }
    }
}

impl Error for PoolProxyError {}

impl From<PoolAdmissionError> for PoolProxyError {
    fn from(error: PoolAdmissionError) -> Self {
        Self::Admission(error)
    }
}

impl From<PoolAuthError> for PoolProxyError {
    fn from(error: PoolAuthError) -> Self {
        Self::Auth(error)
    }
}

impl From<SettingsBucketError> for PoolProxyError {
    fn from(error: SettingsBucketError) -> Self {
        Self::SettingsBucket(error)
    }
}

impl From<GeoIpError> for PoolProxyError {
    fn from(error: GeoIpError) -> Self {
        Self::GeoRouting(error.to_string())
    }
}

impl From<crate::PoolRuntimeError> for PoolProxyError {
    fn from(error: crate::PoolRuntimeError) -> Self {
        Self::SettingsBucket(SettingsBucketError::Runtime(error))
    }
}

impl From<io::Error> for PoolProxyError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<std::net::AddrParseError> for PoolProxyError {
    fn from(error: std::net::AddrParseError) -> Self {
        Self::InvalidAddress(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn geo_routing_config_routes_client_ip_to_replica_and_fallback() {
        let config = PoolGeoRoutingConfig {
            policy: GeoRoutingPolicy {
                default_region: "us-east-1".to_string(),
                rules: vec![GeoRoutingRule {
                    cidr: "127.0.0.0/8".to_string(),
                    region: "moon".to_string(),
                }],
            },
            replicas: ClosestReplicaTable::from_specs(&[
                "us-east-1,1,127.0.0.1,15432",
                "eu-west-1,1,127.0.0.1,25432",
            ])
            .unwrap(),
        };

        let decision = config.route_upstream("127.0.0.1".parse().unwrap()).unwrap();

        assert_eq!(decision.upstream_addr, "127.0.0.1:15432");
        assert_eq!(decision.selected_region, "us-east-1");
        assert!(decision.fallback_used);
    }

    #[test]
    fn config_requires_upstream_and_separate_ports() {
        let config = PoolProxyConfig {
            listen_addr: "127.0.0.1:15432".to_string(),
            admin_addr: "127.0.0.1:15432".to_string(),
            upstream_addr: "127.0.0.1:25432".to_string(),
            client_cidr_allowlist: ClientCidrAllowlist::default(),
            admission: PoolAdmissionConfig::default(),
            auth: None,
            settings_bucket: None,
            geo_routing: None,
        };

        assert_eq!(
            config.validate().unwrap_err(),
            PoolProxyError::AddressCollision {
                left: "listen_addr",
                right: "admin_addr",
            }
        );
    }

    #[test]
    fn admin_readyz_reflects_upstream_connectivity() {
        let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
        let upstream_addr = upstream.local_addr().unwrap().to_string();
        let upstream_thread = thread::spawn(move || {
            let _ = upstream.accept();
        });
        let state = PoolProxyState::new();
        let request = AdminRequest {
            method: "GET".to_string(),
            path: "/readyz".to_string(),
        };

        let response = handle_admin_request(&request, &state, &upstream_addr);

        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"upstream_ready\":true"));
        upstream_thread.join().unwrap();
    }

    #[test]
    fn admin_healthz_does_not_claim_unreachable_upstream_ready() {
        let state = PoolProxyState::new();
        let request = AdminRequest {
            method: "GET".to_string(),
            path: "/healthz".to_string(),
        };

        let response = handle_admin_request(&request, &state, "127.0.0.1:1");

        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"ready\":true"));
        assert!(response.body.contains("\"upstream_ready\":false"));
    }

    #[test]
    fn admin_metrics_expose_pool_counters() {
        let state = PoolProxyState::new();
        state.accepted();
        state.connect_error();
        state.completed();
        let request = AdminRequest {
            method: "GET".to_string(),
            path: "/metrics".to_string(),
        };

        let response = handle_admin_request(&request, &state, "127.0.0.1:1");

        assert_eq!(response.status_code, 200);
        assert!(response
            .body
            .contains("ai_blaise_citus_pool_requests_total 1"));
        assert!(response
            .body
            .contains("ai_blaise_citus_pool_rejected_connections_total 0"));
        assert!(response
            .body
            .contains("ai_blaise_citus_pool_errors_total 1"));
    }

    #[test]
    fn settings_bucket_policy_from_env_is_disabled_without_tracked_gucs() {
        std::env::remove_var("AI_BLAISE_POOL_SETTINGS_BUCKET_GUCS");
        assert_eq!(settings_bucket_policy_from_env().unwrap(), None);
    }

    #[test]
    fn tracked_settings_are_extracted_from_libpq_options() {
        let packet = build_startup_packet_with_options(
            "settings-smoke",
            Some("-c citus.enable_repartition_joins=on -cstatement_timeout=5000"),
        );
        let mut cursor = std::io::Cursor::new(packet);
        let tap = trace_tap::tap_startup_message(&mut cursor).unwrap();
        let settings = tracked_settings_from_startup(
            &tap,
            &[
                "citus.enable_repartition_joins".to_string(),
                "statement_timeout".to_string(),
            ],
        );

        assert_eq!(settings.len(), 2);
        assert_eq!(settings[0].value, "on");
        assert_eq!(settings[1].value, "5000");
    }

    #[test]
    fn settings_bucket_accounting_tracks_and_releases_startup_gucs() {
        let state = PoolProxyState::with_proxy_config(
            PoolAdmissionConfig::default(),
            None,
            Some(SettingsBucketPolicy {
                bucket_name: "test".to_string(),
                tracked_gucs: vec!["citus.enable_repartition_joins".to_string()],
                max_connections: 2,
            }),
        )
        .unwrap();
        let on_packet = build_startup_packet_with_options(
            "settings-smoke",
            Some("-c citus.enable_repartition_joins=on"),
        );
        let off_packet = build_startup_packet_with_options(
            "settings-smoke",
            Some("-c citus.enable_repartition_joins=off"),
        );
        let mut on_cursor = std::io::Cursor::new(on_packet);
        let mut off_cursor = std::io::Cursor::new(off_packet);
        let on_tap = trace_tap::tap_startup_message(&mut on_cursor).unwrap();
        let off_tap = trace_tap::tap_startup_message(&mut off_cursor).unwrap();

        let on_fingerprint = state.acquire_settings_bucket(&on_tap).unwrap().unwrap();
        let off_fingerprint = state.acquire_settings_bucket(&off_tap).unwrap().unwrap();
        assert_ne!(on_fingerprint, off_fingerprint);
        assert_eq!(state.settings_bucket_snapshot(), (2, 2, 2, 0));

        state.release_settings_bucket(&on_fingerprint);
        state.release_settings_bucket(&off_fingerprint);
        assert_eq!(state.settings_bucket_snapshot(), (2, 0, 2, 0));
    }

    #[test]
    fn cidr_allowlist_accepts_matching_clients_and_rejects_others() {
        let allowlist = ClientCidrAllowlist::parse_csv("127.0.0.0/8,10.244.0.0/16").unwrap();

        assert!(allowlist.allows("127.0.0.1".parse().unwrap()));
        assert!(allowlist.allows("10.244.2.15".parse().unwrap()));
        assert!(!allowlist.allows("192.0.2.10".parse().unwrap()));
        assert_eq!(allowlist.as_csv(), "127.0.0.0/8,10.244.0.0/16");
    }

    #[test]
    fn cidr_allowlist_rejects_invalid_prefixes() {
        assert_eq!(
            ClientCidrAllowlist::parse_csv("127.0.0.1/33").unwrap_err(),
            PoolProxyError::InvalidCidr("127.0.0.1/33".to_string())
        );
        assert_eq!(
            ClientCidrAllowlist::parse_csv("not-a-cidr").unwrap_err(),
            PoolProxyError::InvalidCidr("not-a-cidr".to_string())
        );
    }

    fn build_startup_packet(application_name: &str) -> Vec<u8> {
        build_startup_packet_with_options(application_name, None)
    }

    fn build_startup_packet_with_options(application_name: &str, options: Option<&str>) -> Vec<u8> {
        let mut body: Vec<u8> = Vec::new();
        body.extend_from_slice(&196608_u32.to_be_bytes());
        body.extend_from_slice(b"user");
        body.push(0);
        body.extend_from_slice(b"postgres");
        body.push(0);
        body.extend_from_slice(b"database");
        body.push(0);
        body.extend_from_slice(b"postgres");
        body.push(0);
        body.extend_from_slice(b"application_name");
        body.push(0);
        body.extend_from_slice(application_name.as_bytes());
        body.push(0);
        if let Some(options) = options {
            body.extend_from_slice(b"options");
            body.push(0);
            body.extend_from_slice(options.as_bytes());
            body.push(0);
        }
        body.push(0);
        let length = (body.len() + 4) as u32;
        let mut packet = Vec::with_capacity(body.len() + 4);
        packet.extend_from_slice(&length.to_be_bytes());
        packet.extend_from_slice(&body);
        packet
    }

    #[test]
    fn proxy_forwards_bidirectional_tcp_bytes() {
        let startup_packet = build_startup_packet("psql");
        let expected_upstream = startup_packet.clone();
        let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
        let upstream_addr = upstream.local_addr().unwrap().to_string();
        let upstream_thread = thread::spawn(move || {
            let (mut stream, _) = upstream.accept().unwrap();
            let mut received = vec![0_u8; expected_upstream.len()];
            stream.read_exact(&mut received).unwrap();
            assert_eq!(received, expected_upstream);
            stream.write_all(b"AuthenticationOK").unwrap();
        });

        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (client, _) = proxy_listener.accept().unwrap();
            let state = PoolProxyState::new();
            let allowlist = ClientCidrAllowlist::parse_csv("127.0.0.0/8").unwrap();
            handle_proxy_connection(client, &upstream_addr, &allowlist, None, &state).unwrap();
            assert_eq!(state.accepted_connections.load(Ordering::Relaxed), 1);
            assert_eq!(state.completed_connections.load(Ordering::Relaxed), 1);
            assert_eq!(state.traceparent_absent_count(), 1);
            assert_eq!(state.traceparent_tapped_count(), 0);
        });

        let mut client = TcpStream::connect(proxy_addr).unwrap();
        client.write_all(&startup_packet).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut response = vec![0_u8; b"AuthenticationOK".len()];
        client.read_exact(&mut response).unwrap();

        assert_eq!(&response, b"AuthenticationOK");
        upstream_thread.join().unwrap();
        proxy_thread.join().unwrap();
    }

    #[test]
    fn proxy_records_traceparent_when_application_name_embeds_one() {
        const TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let application_name =
            format!("application=ai_blaise_pipeline_smoke;traceparent={TRACEPARENT}");
        let startup_packet = build_startup_packet(&application_name);
        let expected_upstream = startup_packet.clone();
        let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
        let upstream_addr = upstream.local_addr().unwrap().to_string();
        let upstream_thread = thread::spawn(move || {
            let (mut stream, _) = upstream.accept().unwrap();
            let mut received = vec![0_u8; expected_upstream.len()];
            stream.read_exact(&mut received).unwrap();
            assert_eq!(received, expected_upstream);
            stream.write_all(b"AuthenticationOK").unwrap();
        });

        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (client, _) = proxy_listener.accept().unwrap();
            let state = PoolProxyState::new();
            let allowlist = ClientCidrAllowlist::parse_csv("127.0.0.0/8").unwrap();
            handle_proxy_connection(client, &upstream_addr, &allowlist, None, &state).unwrap();
            assert_eq!(state.traceparent_tapped_count(), 1);
            assert_eq!(state.traceparent_absent_count(), 0);
        });

        let mut client = TcpStream::connect(proxy_addr).unwrap();
        client.write_all(&startup_packet).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut response = vec![0_u8; b"AuthenticationOK".len()];
        client.read_exact(&mut response).unwrap();

        assert_eq!(&response, b"AuthenticationOK");
        upstream_thread.join().unwrap();
        proxy_thread.join().unwrap();
    }

    #[test]
    fn proxy_rejects_overload_before_startup_or_upstream_connect() {
        let state = Arc::new(
            PoolProxyState::with_admission_config(PoolAdmissionConfig {
                max_active_connections: Some(1),
                admission_timeout: Duration::ZERO,
                startup_timeout: trace_tap::STARTUP_TAP_MIN_TIMEOUT,
                tenant_quota: None,
            })
            .unwrap(),
        );
        let worker_state = Arc::clone(&state);
        let held_permit = state.admission().acquire_connection().unwrap();
        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (client, _) = proxy_listener.accept().unwrap();
            let allowlist = ClientCidrAllowlist::parse_csv("127.0.0.0/8").unwrap();
            let error =
                handle_proxy_connection(client, "127.0.0.1:1", &allowlist, None, &worker_state)
                    .unwrap_err();

            assert!(matches!(
                error,
                PoolProxyError::Admission(PoolAdmissionError::Overloaded { .. })
            ));
            assert_eq!(worker_state.accepted_connections.load(Ordering::Relaxed), 0);
            assert_eq!(worker_state.rejected_connections.load(Ordering::Relaxed), 1);
            assert_eq!(
                worker_state.overloaded_connections.load(Ordering::Relaxed),
                1
            );
            assert_eq!(
                worker_state.upstream_connect_errors.load(Ordering::Relaxed),
                0
            );
        });

        let _client = TcpStream::connect(proxy_addr).unwrap();
        proxy_thread.join().unwrap();
        drop(held_permit);
        assert_eq!(state.admission().active_slots(), Ok(0));
    }

    #[test]
    fn proxy_fails_closed_when_upstream_is_unreachable() {
        let startup_packet = build_startup_packet("psql");
        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (client, _) = proxy_listener.accept().unwrap();
            let state = PoolProxyState::new();
            let allowlist = ClientCidrAllowlist::parse_csv("127.0.0.0/8").unwrap();
            let error = handle_proxy_connection(client, "127.0.0.1:1", &allowlist, None, &state)
                .unwrap_err();

            assert!(matches!(error, PoolProxyError::Io(_)));
            assert_eq!(state.accepted_connections.load(Ordering::Relaxed), 1);
            assert_eq!(state.completed_connections.load(Ordering::Relaxed), 1);
            assert_eq!(state.upstream_connect_errors.load(Ordering::Relaxed), 1);
            assert_eq!(state.fail_closed_routes.load(Ordering::Relaxed), 1);
        });

        let mut client = TcpStream::connect(proxy_addr).unwrap();
        client.write_all(&startup_packet).unwrap();
        let mut message_type = [0_u8; 1];
        client.read_exact(&mut message_type).unwrap();
        assert_eq!(&message_type, b"E");
        proxy_thread.join().unwrap();
    }

    #[test]
    fn proxy_times_out_incomplete_startup_and_releases_slot() {
        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (client, _) = proxy_listener.accept().unwrap();
            let state = PoolProxyState::with_admission_config(PoolAdmissionConfig {
                max_active_connections: Some(1),
                admission_timeout: Duration::ZERO,
                startup_timeout: trace_tap::STARTUP_TAP_MIN_TIMEOUT,
                tenant_quota: None,
            })
            .unwrap();
            let allowlist = ClientCidrAllowlist::parse_csv("127.0.0.0/8").unwrap();
            let error = handle_proxy_connection(client, "127.0.0.1:1", &allowlist, None, &state)
                .unwrap_err();

            assert!(matches!(error, PoolProxyError::Io(_)));
            assert_eq!(state.accepted_connections.load(Ordering::Relaxed), 1);
            assert_eq!(state.completed_connections.load(Ordering::Relaxed), 1);
            assert_eq!(state.rejected_connections.load(Ordering::Relaxed), 1);
            assert_eq!(state.startup_timeouts.load(Ordering::Relaxed), 1);
            assert_eq!(state.fail_closed_routes.load(Ordering::Relaxed), 1);
            assert_eq!(state.active_connections(), 0);
            assert_eq!(state.admission().active_slots(), Ok(0));
        });

        let _client = TcpStream::connect(proxy_addr).unwrap();
        proxy_thread.join().unwrap();
    }

    #[test]
    fn proxy_rejects_clients_outside_cidr_allowlist_before_upstream_connect() {
        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        let proxy_thread = thread::spawn(move || {
            let (client, _) = proxy_listener.accept().unwrap();
            let state = PoolProxyState::new();
            let allowlist = ClientCidrAllowlist::parse_csv("192.0.2.0/24").unwrap();
            let error = handle_proxy_connection(client, "127.0.0.1:1", &allowlist, None, &state)
                .unwrap_err();

            assert!(matches!(error, PoolProxyError::ClientRejected { .. }));
            assert_eq!(state.rejected_connections.load(Ordering::Relaxed), 1);
            assert_eq!(state.accepted_connections.load(Ordering::Relaxed), 0);
            assert_eq!(state.upstream_connect_errors.load(Ordering::Relaxed), 0);
        });

        let _client = TcpStream::connect(proxy_addr).unwrap();
        proxy_thread.join().unwrap();
    }

    /// Encoded simple-query frame `Q [length] body\0`.
    fn pack_simple_query(query: &str) -> Vec<u8> {
        let mut body = query.as_bytes().to_vec();
        body.push(0);
        let mut frame = vec![b'Q'];
        frame.extend_from_slice(&((body.len() + 4) as u32).to_be_bytes());
        frame.extend_from_slice(&body);
        frame
    }

    #[test]
    fn forward_client_to_upstream_counts_known_frames_and_forwards_bytes() {
        // Sanity-check the helper that packs simple-query frames matches the
        // wire crate's QueryFrame encoder (catch silent format drift).
        let mut codec_buf = ai_blaise_citus_pool_wire::PgWriteBuf::new();
        ai_blaise_citus_pool_wire::QueryFrame {
            query: "SELECT 1".to_string(),
        }
        .encode(&mut codec_buf);
        assert_eq!(codec_buf.into_inner(), pack_simple_query("SELECT 1"));

        // Set up a real loopback pair: forwarder reads from one end, writes
        // to the upstream end. Run it on a thread; main thread sends frames.
        let upstream_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let upstream_addr = upstream_listener.local_addr().unwrap();
        let client_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client_addr = client_listener.local_addr().unwrap();
        let state = Arc::new(
            PoolProxyState::with_admission_config(PoolAdmissionConfig::default()).unwrap(),
        );
        let state_for_thread = Arc::clone(&state);

        let forwarder_thread = thread::spawn(move || {
            let (client_socket, _) = client_listener.accept().unwrap();
            let mut upstream_socket = TcpStream::connect(upstream_addr).unwrap();
            let mut reader = client_socket;
            forward_client_to_upstream(&mut reader, &mut upstream_socket, &state_for_thread)
                .unwrap();
        });

        // Capture every byte the upstream receives.
        let upstream_capture = thread::spawn(move || {
            let (mut socket, _) = upstream_listener.accept().unwrap();
            let mut buf = Vec::new();
            socket.read_to_end(&mut buf).unwrap();
            buf
        });

        let mut client = TcpStream::connect(client_addr).unwrap();
        // Two Q frames + one X (Terminate) - all known frontend tags.
        let mut script = pack_simple_query("SELECT 1");
        script.extend_from_slice(&pack_simple_query("SELECT 2"));
        // Terminate: 'X' [length=4]
        script.extend_from_slice(b"X");
        script.extend_from_slice(&4_u32.to_be_bytes());
        client.write_all(&script).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        forwarder_thread.join().unwrap();
        let captured = upstream_capture.join().unwrap();

        assert_eq!(captured, script, "forwarder must preserve byte stream");
        let counters = state.ext_query_counters();
        assert_eq!(counters.simple_query, 2, "two Q frames");
        assert_eq!(counters.terminate, 1, "one X frame");
        assert_eq!(counters.decode_errors, 0, "no decode errors on known tags");
    }

    #[test]
    fn forward_client_to_upstream_falls_back_on_unknown_tag() {
        let upstream_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let upstream_addr = upstream_listener.local_addr().unwrap();
        let client_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client_addr = client_listener.local_addr().unwrap();
        let state = Arc::new(
            PoolProxyState::with_admission_config(PoolAdmissionConfig::default()).unwrap(),
        );
        let state_for_thread = Arc::clone(&state);

        let forwarder_thread = thread::spawn(move || {
            let (client_socket, _) = client_listener.accept().unwrap();
            let mut upstream_socket = TcpStream::connect(upstream_addr).unwrap();
            let mut reader = client_socket;
            forward_client_to_upstream(&mut reader, &mut upstream_socket, &state_for_thread)
                .unwrap();
        });

        let upstream_capture = thread::spawn(move || {
            let (mut socket, _) = upstream_listener.accept().unwrap();
            let mut buf = Vec::new();
            socket.read_to_end(&mut buf).unwrap();
            buf
        });

        let mut client = TcpStream::connect(client_addr).unwrap();
        // 0x00 is not a known frontend tag - resembles a second StartupMessage
        // length prefix after an SSL/GSS exchange. The forwarder must fall
        // back to byte-copy and forward every byte verbatim.
        let payload = vec![0x00, 0x00, 0x00, 0x08, 0x04, 0xd2, 0x16, 0x2f];
        client.write_all(&payload).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        forwarder_thread.join().unwrap();
        let captured = upstream_capture.join().unwrap();

        assert_eq!(captured, payload, "byte-copy fallback must preserve bytes");
        let counters = state.ext_query_counters();
        assert_eq!(counters.decode_errors, 1, "one decode error on unknown tag");
        assert_eq!(counters.parse, 0);
        assert_eq!(counters.bind, 0);
    }

    #[test]
    fn forward_client_to_upstream_falls_back_on_invalid_length() {
        let upstream_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let upstream_addr = upstream_listener.local_addr().unwrap();
        let client_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client_addr = client_listener.local_addr().unwrap();
        let state = Arc::new(
            PoolProxyState::with_admission_config(PoolAdmissionConfig::default()).unwrap(),
        );
        let state_for_thread = Arc::clone(&state);

        let forwarder_thread = thread::spawn(move || {
            let (client_socket, _) = client_listener.accept().unwrap();
            let mut upstream_socket = TcpStream::connect(upstream_addr).unwrap();
            let mut reader = client_socket;
            forward_client_to_upstream(&mut reader, &mut upstream_socket, &state_for_thread)
                .unwrap();
        });

        let upstream_capture = thread::spawn(move || {
            let (mut socket, _) = upstream_listener.accept().unwrap();
            let mut buf = Vec::new();
            socket.read_to_end(&mut buf).unwrap();
            buf
        });

        let mut client = TcpStream::connect(client_addr).unwrap();
        // Tag 'Q' with declared length 2 (< 4 minimum). FrameHeader::read
        // rejects this as InvalidLength; the forwarder must fall back to
        // byte-copy.
        let payload = vec![b'Q', 0x00, 0x00, 0x00, 0x02, 0xAA, 0xBB];
        client.write_all(&payload).unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        forwarder_thread.join().unwrap();
        let captured = upstream_capture.join().unwrap();

        assert_eq!(captured, payload, "byte-copy must preserve bytes after invalid length");
        let counters = state.ext_query_counters();
        assert_eq!(counters.decode_errors, 1);
        assert_eq!(counters.simple_query, 0);
    }
}
