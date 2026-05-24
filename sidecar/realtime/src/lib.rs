//! Realtime sidecar contracts and real-runtime entry points.
//!
//! Layers:
//! 1. Contract data structures ([`RealtimeSidecarPlan`],
//!    [`RealtimeSubscription`], [`PresencePlan`]) with deterministic
//!    `broadcast_plan` semantics.
//! 2. WebSocket primitives ([`ws`]) implementing the RFC 6455 handshake +
//!    text frame parsing in pure `std`.
//! 3. Phoenix Channels frame encode/decode ([`phoenix`]).
//! 4. In-process fan-out hub + presence ([`hub`]).
//! 5. Live runtime ([`live`]) that wires the WS server, CDC ingest, and
//!    the hub together.

// FEATURE: RT1
// FEATURE: RT2
// FEATURE: RT3
// FEATURE: RT4
// FEATURE: RT5

pub mod hub;
pub mod live;
pub mod phoenix;
pub mod ws;

pub use hub::{
    HubMetrics, Mailbox, PresenceEntry, PresenceState, RealtimeHub, Subscription,
    SubscriptionFilter,
};
pub use live::{
    handle_cdc_ingest, handle_ws_connection, RealtimeLiveConfig, RealtimeLiveRuntime,
    MAX_CDC_INGEST_FRAME_BYTES,
};
pub use phoenix::{PhoenixDecodeError, PhoenixFrame};
pub use ws::{
    decode_frame, encode_close_frame, encode_text_frame, UpgradeRequest, WsConnection, WsError,
    MAX_WS_TEXT_FRAME_BYTES,
};

use ai_blaise_citus_sidecar_cdc::{
    CdcColumnValue, CdcEventEnvelope, CdcOperation, CdcSidecarError,
};
use ai_blaise_citus_sidecar_shared::{RealtimeContract, SidecarContractError};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeSidecarPlan {
    pub contract: RealtimeContract,
    pub source_slot: String,
    pub subscriptions: Vec<RealtimeSubscription>,
    pub presence: Option<PresencePlan>,
}

impl RealtimeSidecarPlan {
    pub fn validate(&self) -> Result<(), RealtimeSidecarError> {
        self.contract.validate()?;
        validate_required("source_slot", &self.source_slot)?;
        if self.subscriptions.is_empty() {
            return Err(RealtimeSidecarError::MissingRequiredField("subscriptions"));
        }
        for subscription in &self.subscriptions {
            subscription.validate()?;
        }
        if let Some(presence) = &self.presence {
            presence.validate()?;
        }
        Ok(())
    }

    pub fn broadcast_plan(
        &self,
        event: &CdcEventEnvelope,
    ) -> Result<RealtimeBroadcastPlan, RealtimeSidecarError> {
        self.validate()?;
        event.validate()?;

        let mut recipients = Vec::new();
        for subscription in &self.subscriptions {
            if subscription.accepts_event(&self.contract, event) {
                recipients.push(subscription.connection_id.clone());
            }
        }

        Ok(RealtimeBroadcastPlan {
            topic: self.contract.topic.clone(),
            tenant_id: event.tenant_id.clone(),
            operation: event.operation,
            recipients,
            presence_snapshot: self.presence.as_ref().map(PresenceSnapshot::from),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeSubscription {
    pub connection_id: String,
    pub user_id: String,
    pub tenant_id: String,
    pub topic: String,
    pub filters: Vec<RealtimeFilter>,
}

impl RealtimeSubscription {
    fn validate(&self) -> Result<(), RealtimeSidecarError> {
        validate_required("subscription.connection_id", &self.connection_id)?;
        validate_required("subscription.user_id", &self.user_id)?;
        validate_required("subscription.tenant_id", &self.tenant_id)?;
        validate_required("subscription.topic", &self.topic)?;
        for filter in &self.filters {
            filter.validate()?;
        }
        Ok(())
    }

    fn accepts_event(&self, contract: &RealtimeContract, event: &CdcEventEnvelope) -> bool {
        self.tenant_id == event.tenant_id
            && self.tenant_id == contract.tenant_id
            && self.topic == contract.topic
            && self
                .filters
                .iter()
                .all(|filter| filter.matches(&event.columns))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeFilter {
    pub column: String,
    pub operator: FilterOperator,
    pub value: String,
}

impl RealtimeFilter {
    fn validate(&self) -> Result<(), RealtimeSidecarError> {
        validate_required("filter.column", &self.column)?;
        validate_required("filter.value", &self.value)
    }

    fn matches(&self, columns: &[CdcColumnValue]) -> bool {
        let Some(column) = columns.iter().find(|column| column.name == self.column) else {
            return false;
        };
        let Some(value) = &column.value else {
            return self.operator == FilterOperator::IsNull;
        };

        match self.operator {
            FilterOperator::Eq => value == &self.value,
            FilterOperator::Ne => value != &self.value,
            FilterOperator::Prefix => value.starts_with(&self.value),
            FilterOperator::Contains => value.contains(&self.value),
            FilterOperator::IsNull => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FilterOperator {
    Eq,
    Ne,
    Prefix,
    Contains,
    IsNull,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PresencePlan {
    pub tenant_id: String,
    pub topic: String,
    pub users: Vec<PresenceUser>,
}

impl PresencePlan {
    fn validate(&self) -> Result<(), RealtimeSidecarError> {
        validate_required("presence.tenant_id", &self.tenant_id)?;
        validate_required("presence.topic", &self.topic)?;
        for user in &self.users {
            user.validate()?;
            if user.tenant_id != self.tenant_id {
                return Err(RealtimeSidecarError::TenantMismatch);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PresenceUser {
    pub user_id: String,
    pub tenant_id: String,
    pub connection_id: String,
    pub online_at: String,
}

impl PresenceUser {
    fn validate(&self) -> Result<(), RealtimeSidecarError> {
        validate_required("presence.user_id", &self.user_id)?;
        validate_required("presence.tenant_id", &self.tenant_id)?;
        validate_required("presence.connection_id", &self.connection_id)?;
        if self.online_at.len() >= 20
            && self.online_at.contains('T')
            && self.online_at.ends_with('Z')
        {
            Ok(())
        } else {
            Err(RealtimeSidecarError::InvalidTimestamp)
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeBroadcastPlan {
    pub topic: String,
    pub tenant_id: String,
    pub operation: CdcOperation,
    pub recipients: Vec<String>,
    pub presence_snapshot: Option<PresenceSnapshot>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PresenceSnapshot {
    pub topic: String,
    pub tenant_id: String,
    pub online_users: Vec<String>,
}

impl From<&PresencePlan> for PresenceSnapshot {
    fn from(plan: &PresencePlan) -> Self {
        Self {
            topic: plan.topic.clone(),
            tenant_id: plan.tenant_id.clone(),
            online_users: plan.users.iter().map(|user| user.user_id.clone()).collect(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RealtimeSidecarError {
    CdcContract(String),
    InvalidTimestamp,
    MissingRequiredField(&'static str),
    RecipientConnectionMissing,
    SharedContract(String),
    TenantMismatch,
}

impl fmt::Display for RealtimeSidecarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CdcContract(error) => write!(formatter, "{error}"),
            Self::InvalidTimestamp => {
                write!(formatter, "online_at must be an RFC3339 UTC timestamp")
            }
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
            Self::RecipientConnectionMissing => {
                write!(
                    formatter,
                    "broadcast recipient did not match an active connection"
                )
            }
            Self::SharedContract(error) => write!(formatter, "{error}"),
            Self::TenantMismatch => {
                write!(formatter, "presence user tenant must match presence tenant")
            }
        }
    }
}

impl Error for RealtimeSidecarError {}

impl From<SidecarContractError> for RealtimeSidecarError {
    fn from(error: SidecarContractError) -> Self {
        Self::SharedContract(error.to_string())
    }
}

impl From<CdcSidecarError> for RealtimeSidecarError {
    fn from(error: CdcSidecarError) -> Self {
        Self::CdcContract(error.to_string())
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), RealtimeSidecarError> {
    if value.trim().is_empty() {
        return Err(RealtimeSidecarError::MissingRequiredField(field));
    }
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeConnection {
    pub connection_id: String,
    pub user_id: String,
    pub tenant_id: String,
    pub topic: String,
}

impl From<&RealtimeSubscription> for RealtimeConnection {
    fn from(subscription: &RealtimeSubscription) -> Self {
        Self {
            connection_id: subscription.connection_id.clone(),
            user_id: subscription.user_id.clone(),
            tenant_id: subscription.tenant_id.clone(),
            topic: subscription.topic.clone(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeDelivery {
    pub connection_id: String,
    pub user_id: String,
    pub tenant_id: String,
    pub topic: String,
    pub frame_bytes: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeRuntimeBroadcast {
    pub topic: String,
    pub tenant_id: String,
    pub operation: CdcOperation,
    pub deliveries: Vec<RealtimeDelivery>,
    pub filtered_connections: usize,
    pub presence_users: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeRuntimeState {
    pub active_connections: usize,
    pub broadcasts: u64,
    pub delivered_messages: u64,
    pub presence_snapshots: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeRuntimeReport {
    pub broadcast: RealtimeRuntimeBroadcast,
    pub state: RealtimeRuntimeState,
    pub runtime_boundary: &'static str,
    pub websocket_network_exercised: bool,
    pub cdc_tailing_integrated: bool,
    pub multi_node_pubsub: bool,
    pub kubernetes_traffic_exercised: bool,
    pub browser_client_exercised: bool,
    pub max_ws_text_frame_bytes: usize,
    pub max_cdc_ingest_frame_bytes: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealtimeRuntime {
    plan: RealtimeSidecarPlan,
    connections: Vec<RealtimeConnection>,
    state: RealtimeRuntimeState,
}

impl RealtimeRuntime {
    pub fn new(plan: RealtimeSidecarPlan) -> Result<Self, RealtimeSidecarError> {
        plan.validate()?;
        let connections = plan
            .subscriptions
            .iter()
            .map(RealtimeConnection::from)
            .collect::<Vec<_>>();

        Ok(Self {
            state: RealtimeRuntimeState {
                active_connections: connections.len(),
                broadcasts: 0,
                delivered_messages: 0,
                presence_snapshots: 0,
            },
            plan,
            connections,
        })
    }

    pub fn state(&self) -> &RealtimeRuntimeState {
        &self.state
    }

    pub fn dispatch_cdc_event(
        &mut self,
        event: &CdcEventEnvelope,
    ) -> Result<RealtimeRuntimeBroadcast, RealtimeSidecarError> {
        let plan = self.plan.broadcast_plan(event)?;
        let mut deliveries = Vec::with_capacity(plan.recipients.len());
        for recipient in &plan.recipients {
            let connection = self
                .connections
                .iter()
                .find(|connection| connection.connection_id == *recipient)
                .ok_or(RealtimeSidecarError::RecipientConnectionMissing)?;
            deliveries.push(RealtimeDelivery {
                connection_id: connection.connection_id.clone(),
                user_id: connection.user_id.clone(),
                tenant_id: connection.tenant_id.clone(),
                topic: connection.topic.clone(),
                frame_bytes: deterministic_frame_bytes(event),
            });
        }

        let presence_users = plan
            .presence_snapshot
            .as_ref()
            .map(|presence| presence.online_users.clone())
            .unwrap_or_default();

        self.state.broadcasts += 1;
        self.state.delivered_messages += deliveries.len() as u64;
        if !presence_users.is_empty() {
            self.state.presence_snapshots += 1;
        }

        Ok(RealtimeRuntimeBroadcast {
            topic: plan.topic,
            tenant_id: plan.tenant_id,
            operation: plan.operation,
            filtered_connections: self.connections.len().saturating_sub(deliveries.len()),
            deliveries,
            presence_users,
        })
    }
}

fn deterministic_frame_bytes(event: &CdcEventEnvelope) -> u64 {
    128 + (event.columns.len() as u64 * 16)
}

pub fn canonical_realtime_plan() -> RealtimeSidecarPlan {
    RealtimeSidecarPlan {
        contract: RealtimeContract {
            topic: "orders".to_string(),
            tenant_id: "tenant-a".to_string(),
            filters: vec!["status=paid".to_string()],
            presence_enabled: true,
        },
        source_slot: "ai_blaise_cdc".to_string(),
        subscriptions: vec![
            RealtimeSubscription {
                connection_id: "conn-a".to_string(),
                user_id: "user-a".to_string(),
                tenant_id: "tenant-a".to_string(),
                topic: "orders".to_string(),
                filters: vec![RealtimeFilter {
                    column: "status".to_string(),
                    operator: FilterOperator::Eq,
                    value: "paid".to_string(),
                }],
            },
            RealtimeSubscription {
                connection_id: "conn-b".to_string(),
                user_id: "user-b".to_string(),
                tenant_id: "tenant-b".to_string(),
                topic: "orders".to_string(),
                filters: vec![RealtimeFilter {
                    column: "status".to_string(),
                    operator: FilterOperator::Eq,
                    value: "paid".to_string(),
                }],
            },
            RealtimeSubscription {
                connection_id: "conn-c".to_string(),
                user_id: "user-c".to_string(),
                tenant_id: "tenant-a".to_string(),
                topic: "orders".to_string(),
                filters: vec![RealtimeFilter {
                    column: "status".to_string(),
                    operator: FilterOperator::Eq,
                    value: "pending".to_string(),
                }],
            },
        ],
        presence: Some(PresencePlan {
            tenant_id: "tenant-a".to_string(),
            topic: "orders".to_string(),
            users: vec![PresenceUser {
                user_id: "user-a".to_string(),
                tenant_id: "tenant-a".to_string(),
                connection_id: "conn-a".to_string(),
                online_at: "2026-05-19T12:00:00Z".to_string(),
            }],
        }),
    }
}

pub fn canonical_realtime_event() -> CdcEventEnvelope {
    CdcEventEnvelope {
        lsn: "16/B374D848".to_string(),
        schema: "public".to_string(),
        table: "orders".to_string(),
        tenant_id: "tenant-a".to_string(),
        operation: CdcOperation::Insert,
        columns: vec![
            CdcColumnValue {
                name: "id".to_string(),
                value: Some("1".to_string()),
            },
            CdcColumnValue {
                name: "status".to_string(),
                value: Some("paid".to_string()),
            },
        ],
    }
}

pub fn canonical_broadcast_plan() -> Result<RealtimeBroadcastPlan, RealtimeSidecarError> {
    canonical_realtime_plan().broadcast_plan(&canonical_realtime_event())
}

pub fn canonical_realtime_runtime_report() -> Result<RealtimeRuntimeReport, RealtimeSidecarError> {
    let mut runtime = RealtimeRuntime::new(canonical_realtime_plan())?;
    let broadcast = runtime.dispatch_cdc_event(&canonical_realtime_event())?;

    Ok(RealtimeRuntimeReport {
        broadcast,
        state: runtime.state().clone(),
        runtime_boundary: "single-node-raw-ws-cdc-ingest",
        websocket_network_exercised: true,
        cdc_tailing_integrated: false,
        multi_node_pubsub: false,
        kubernetes_traffic_exercised: false,
        browser_client_exercised: false,
        max_ws_text_frame_bytes: MAX_WS_TEXT_FRAME_BYTES,
        max_cdc_ingest_frame_bytes: MAX_CDC_INGEST_FRAME_BYTES,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realtime_plan_routes_matching_tenant_and_filter() {
        let plan = canonical_realtime_plan();
        let broadcast = plan
            .broadcast_plan(&valid_event("tenant-a", "paid"))
            .expect("broadcast plan");

        assert_eq!(broadcast.topic, "orders");
        assert_eq!(broadcast.tenant_id, "tenant-a");
        assert_eq!(broadcast.recipients, vec!["conn-a".to_string()]);
        assert_eq!(
            broadcast.presence_snapshot.expect("presence").online_users,
            vec!["user-a".to_string()]
        );
    }

    #[test]
    fn canonical_broadcast_plan_is_deterministic() {
        let broadcast = canonical_broadcast_plan().expect("canonical broadcast");

        assert_eq!(broadcast.topic, "orders");
        assert_eq!(broadcast.tenant_id, "tenant-a");
        assert_eq!(broadcast.recipients, vec!["conn-a".to_string()]);
        assert_eq!(
            broadcast.presence_snapshot.expect("presence").online_users,
            vec!["user-a".to_string()]
        );
    }

    #[test]
    fn realtime_runtime_dispatches_to_matching_connection() {
        let report = canonical_realtime_runtime_report().expect("runtime report");

        assert_eq!(report.broadcast.topic, "orders");
        assert_eq!(report.broadcast.tenant_id, "tenant-a");
        assert_eq!(report.broadcast.deliveries.len(), 1);
        assert_eq!(report.broadcast.deliveries[0].connection_id, "conn-a");
        assert_eq!(report.broadcast.deliveries[0].user_id, "user-a");
        assert_eq!(report.broadcast.deliveries[0].frame_bytes, 160);
        assert_eq!(report.broadcast.filtered_connections, 2);
        assert_eq!(report.broadcast.presence_users, vec!["user-a".to_string()]);
        assert_eq!(report.state.active_connections, 3);
        assert_eq!(report.state.broadcasts, 1);
        assert_eq!(report.state.delivered_messages, 1);
        assert_eq!(report.state.presence_snapshots, 1);
        assert_eq!(report.runtime_boundary, "single-node-raw-ws-cdc-ingest");
        assert!(report.websocket_network_exercised);
        assert!(!report.cdc_tailing_integrated);
        assert!(!report.multi_node_pubsub);
        assert!(!report.kubernetes_traffic_exercised);
        assert!(!report.browser_client_exercised);
        assert_eq!(report.max_ws_text_frame_bytes, 64 * 1024);
        assert_eq!(report.max_cdc_ingest_frame_bytes, 1 << 20);
    }

    #[test]
    fn realtime_runtime_tracks_filtered_event() {
        let mut runtime = RealtimeRuntime::new(canonical_realtime_plan()).expect("runtime");
        let broadcast = runtime
            .dispatch_cdc_event(&valid_event("tenant-a", "refunded"))
            .expect("broadcast");

        assert!(broadcast.deliveries.is_empty());
        assert_eq!(broadcast.filtered_connections, 3);
        assert_eq!(runtime.state().broadcasts, 1);
        assert_eq!(runtime.state().delivered_messages, 0);
        assert_eq!(runtime.state().presence_snapshots, 1);
    }

    #[test]
    fn realtime_plan_blocks_cross_tenant_subscriptions() {
        let plan = canonical_realtime_plan();
        let broadcast = plan
            .broadcast_plan(&valid_event("tenant-b", "paid"))
            .expect("broadcast plan");

        assert!(broadcast.recipients.is_empty());
    }

    #[test]
    fn realtime_filter_blocks_non_matching_events() {
        let plan = canonical_realtime_plan();
        let broadcast = plan
            .broadcast_plan(&valid_event("tenant-a", "refunded"))
            .expect("broadcast plan");

        assert!(broadcast.recipients.is_empty());
    }

    #[test]
    fn presence_rejects_cross_tenant_user() {
        let mut plan = canonical_realtime_plan();
        plan.presence = Some(PresencePlan {
            tenant_id: "tenant-a".to_string(),
            topic: "orders".to_string(),
            users: vec![PresenceUser {
                user_id: "user-b".to_string(),
                tenant_id: "tenant-b".to_string(),
                connection_id: "conn-b".to_string(),
                online_at: "2026-05-19T12:00:00Z".to_string(),
            }],
        });

        assert_eq!(plan.validate(), Err(RealtimeSidecarError::TenantMismatch));
    }

    #[test]
    fn presence_requires_utc_timestamp() {
        let user = PresenceUser {
            user_id: "user-a".to_string(),
            tenant_id: "tenant-a".to_string(),
            connection_id: "conn-a".to_string(),
            online_at: "2026-05-19 12:00:00".to_string(),
        };

        assert_eq!(user.validate(), Err(RealtimeSidecarError::InvalidTimestamp));
    }

    fn valid_event(tenant_id: &str, status: &str) -> CdcEventEnvelope {
        CdcEventEnvelope {
            lsn: "16/B374D848".to_string(),
            schema: "public".to_string(),
            table: "orders".to_string(),
            tenant_id: tenant_id.to_string(),
            operation: CdcOperation::Insert,
            columns: vec![
                CdcColumnValue {
                    name: "id".to_string(),
                    value: Some("1".to_string()),
                },
                CdcColumnValue {
                    name: "status".to_string(),
                    value: Some(status.to_string()),
                },
            ],
        }
    }
}
