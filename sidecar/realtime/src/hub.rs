//! In-process fan-out hub for the realtime sidecar.
//!
//! Subscriptions are keyed by topic + tenant. The CDC ingest path calls
//! [`RealtimeHub::broadcast`] with a `CdcEventEnvelope` and the hub
//! distributes the encoded Phoenix frame to every matching subscription's
//! mailbox. The WS handler drains its mailbox each tick and writes the
//! frames to the socket.

// FEATURE: RT1
// FEATURE: RT2
// FEATURE: RT3
// FEATURE: RT4
// FEATURE: RT5

use crate::phoenix::PhoenixFrame;
use ai_blaise_citus_sidecar_cdc::{CdcColumnValue, CdcEventEnvelope, CdcOperation};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

static SUBSCRIPTION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SubscriptionFilter {
    pub schema: String,
    pub table: String,
    pub op: Option<CdcOperation>,
    pub equals: HashMap<String, String>,
}

impl SubscriptionFilter {
    pub fn matches(&self, event: &CdcEventEnvelope) -> bool {
        if self.schema != event.schema || self.table != event.table {
            return false;
        }
        if let Some(op) = self.op {
            if op != event.operation {
                return false;
            }
        }
        for (column, expected) in &self.equals {
            let Some(found) = event.columns.iter().find(|c| c.name == *column) else {
                return false;
            };
            if found.value.as_deref() != Some(expected.as_str()) {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Subscription {
    pub id: u64,
    pub connection_id: String,
    pub user_id: String,
    pub tenant_id: String,
    pub topic: String,
    pub filter: SubscriptionFilter,
}

#[derive(Debug)]
pub struct Mailbox {
    inner: Mutex<VecDeque<String>>,
}

impl Mailbox {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(VecDeque::new()),
        }
    }

    pub fn push(&self, frame: String) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.push_back(frame);
        }
    }

    pub fn drain(&self) -> Vec<String> {
        match self.inner.lock() {
            Ok(mut guard) => guard.drain(..).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.lock().map(|guard| guard.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for Mailbox {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PresenceEntry {
    pub user_id: String,
    pub connection_id: String,
    pub metadata: Value,
    pub online_at: String,
}

#[derive(Debug, Default)]
pub struct PresenceState {
    entries: HashMap<String, Vec<PresenceEntry>>,
}

impl PresenceState {
    pub fn join(&mut self, topic: &str, entry: PresenceEntry) -> Value {
        let topic_entries = self.entries.entry(topic.to_string()).or_default();
        topic_entries.retain(|existing| existing.connection_id != entry.connection_id);
        topic_entries.push(entry.clone());
        json!({
            "joins": {
                entry.user_id.clone(): {
                    "metas": [{
                        "online_at": entry.online_at,
                        "phx_ref": entry.connection_id,
                        "metadata": entry.metadata,
                    }]
                }
            },
            "leaves": {},
        })
    }

    pub fn leave(&mut self, topic: &str, connection_id: &str) -> Value {
        let topic_entries = self.entries.entry(topic.to_string()).or_default();
        let removed: Vec<PresenceEntry> = topic_entries
            .iter()
            .filter(|entry| entry.connection_id == connection_id)
            .cloned()
            .collect();
        topic_entries.retain(|entry| entry.connection_id != connection_id);
        if removed.is_empty() {
            return json!({"joins": {}, "leaves": {}});
        }
        let mut leaves = serde_json::Map::new();
        for entry in &removed {
            leaves.insert(
                entry.user_id.clone(),
                json!({"metas": [{
                    "online_at": entry.online_at,
                    "phx_ref": entry.connection_id,
                    "metadata": entry.metadata,
                }]}),
            );
        }
        json!({"joins": {}, "leaves": leaves})
    }

    pub fn snapshot(&self, topic: &str) -> Value {
        let mut state = serde_json::Map::new();
        if let Some(entries) = self.entries.get(topic) {
            for entry in entries {
                let metas = state
                    .entry(entry.user_id.clone())
                    .or_insert_with(|| json!({"metas": []}));
                metas["metas"].as_array_mut().expect("metas").push(json!({
                    "online_at": entry.online_at,
                    "phx_ref": entry.connection_id,
                    "metadata": entry.metadata,
                }));
            }
        }
        Value::Object(state)
    }

    pub fn topic_user_count(&self, topic: &str) -> usize {
        self.entries
            .get(topic)
            .map(|entries| entries.len())
            .unwrap_or(0)
    }
}

#[derive(Debug)]
pub struct RealtimeHub {
    subscriptions: Mutex<Vec<(Subscription, std::sync::Arc<Mailbox>)>>,
    presence: Mutex<PresenceState>,
    metrics: Mutex<HubMetrics>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct HubMetrics {
    pub broadcasts: u64,
    pub delivered: u64,
    pub filtered: u64,
    pub presence_joins: u64,
    pub presence_leaves: u64,
}

impl RealtimeHub {
    pub fn new() -> Self {
        Self {
            subscriptions: Mutex::new(Vec::new()),
            presence: Mutex::new(PresenceState::default()),
            metrics: Mutex::new(HubMetrics::default()),
        }
    }

    pub fn metrics(&self) -> HubMetrics {
        self.metrics
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions
            .lock()
            .map(|guard| guard.iter().map(|(sub, _)| sub.clone()).collect())
            .unwrap_or_default()
    }

    pub fn subscription_count(&self) -> usize {
        self.subscriptions
            .lock()
            .map(|guard| guard.len())
            .unwrap_or(0)
    }

    pub fn subscribe(
        &self,
        connection_id: String,
        user_id: String,
        tenant_id: String,
        topic: String,
        filter: SubscriptionFilter,
    ) -> (Subscription, std::sync::Arc<Mailbox>) {
        let id = SUBSCRIPTION_COUNTER.fetch_add(1, Ordering::SeqCst);
        let mailbox = std::sync::Arc::new(Mailbox::new());
        let subscription = Subscription {
            id,
            connection_id,
            user_id,
            tenant_id,
            topic,
            filter,
        };
        if let Ok(mut guard) = self.subscriptions.lock() {
            guard.push((subscription.clone(), mailbox.clone()));
        }
        (subscription, mailbox)
    }

    pub fn unsubscribe(&self, subscription_id: u64) {
        if let Ok(mut guard) = self.subscriptions.lock() {
            guard.retain(|(sub, _)| sub.id != subscription_id);
        }
    }

    pub fn unsubscribe_connection(&self, connection_id: &str) -> Vec<Subscription> {
        let mut removed = Vec::new();
        if let Ok(mut guard) = self.subscriptions.lock() {
            guard.retain(|(sub, _)| {
                if sub.connection_id == connection_id {
                    removed.push(sub.clone());
                    false
                } else {
                    true
                }
            });
        }
        removed
    }

    pub fn presence_join(
        &self,
        topic: &str,
        user_id: String,
        connection_id: String,
        metadata: Value,
        online_at: String,
    ) -> Value {
        let entry = PresenceEntry {
            user_id,
            connection_id,
            metadata,
            online_at,
        };
        let diff = self
            .presence
            .lock()
            .map(|mut state| state.join(topic, entry))
            .unwrap_or(Value::Null);
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.presence_joins += 1;
        }
        diff
    }

    pub fn presence_leave(&self, topic: &str, connection_id: &str) -> Value {
        let diff = self
            .presence
            .lock()
            .map(|mut state| state.leave(topic, connection_id))
            .unwrap_or(Value::Null);
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.presence_leaves += 1;
        }
        diff
    }

    pub fn presence_snapshot(&self, topic: &str) -> Value {
        self.presence
            .lock()
            .map(|state| state.snapshot(topic))
            .unwrap_or(Value::Null)
    }

    pub fn presence_user_count(&self, topic: &str) -> usize {
        self.presence
            .lock()
            .map(|state| state.topic_user_count(topic))
            .unwrap_or(0)
    }

    /// Push the encoded Phoenix frame for `event` to every subscription
    /// whose filter matches. Returns the number of mailboxes that received
    /// the frame.
    pub fn broadcast(&self, event: &CdcEventEnvelope) -> usize {
        let frame = encode_postgres_changes(event);
        let mut delivered = 0;
        let mut filtered = 0;
        let subscriptions = self
            .subscriptions
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        for (subscription, mailbox) in &subscriptions {
            if subscription.tenant_id != event.tenant_id {
                filtered += 1;
                continue;
            }
            if !subscription.filter.matches(event) {
                filtered += 1;
                continue;
            }
            let topic_frame = PhoenixFrame {
                join_ref: None,
                message_ref: None,
                topic: subscription.topic.clone(),
                event: "postgres_changes".to_string(),
                payload: frame.clone(),
            };
            mailbox.push(topic_frame.encode());
            delivered += 1;
        }
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.broadcasts += 1;
            metrics.delivered += delivered as u64;
            metrics.filtered += filtered as u64;
        }
        delivered
    }
}

impl Default for RealtimeHub {
    fn default() -> Self {
        Self::new()
    }
}

fn encode_postgres_changes(event: &CdcEventEnvelope) -> Value {
    let mut columns = Vec::with_capacity(event.columns.len());
    for column in &event.columns {
        let value = column
            .value
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null);
        columns.push(json!({"name": column.name, "value": value}));
    }
    json!({
        "schema": event.schema,
        "table": event.table,
        "tenant_id": event.tenant_id,
        "type": operation_token(event.operation),
        "lsn": event.lsn,
        "columns": columns,
    })
}

fn operation_token(op: CdcOperation) -> &'static str {
    match op {
        CdcOperation::Insert => "INSERT",
        CdcOperation::Update => "UPDATE",
        CdcOperation::Delete => "DELETE",
        CdcOperation::Truncate => "TRUNCATE",
    }
}

/// Helper to suppress an unused import warning when the [`CdcColumnValue`]
/// type is only referenced indirectly through the broadcast payload.
#[allow(dead_code)]
fn _force_link(value: &CdcColumnValue) -> &str {
    &value.name
}

#[cfg(test)]
mod tests {
    use super::*;
    use ai_blaise_citus_sidecar_cdc::canonical_cdc_event;

    #[test]
    fn broadcast_delivers_to_matching_subscriber() {
        let hub = RealtimeHub::new();
        let (_sub, mailbox) = hub.subscribe(
            "conn-a".to_string(),
            "user-a".to_string(),
            "tenant-a".to_string(),
            "realtime:public:orders".to_string(),
            SubscriptionFilter {
                schema: "public".to_string(),
                table: "orders".to_string(),
                op: Some(CdcOperation::Insert),
                equals: HashMap::new(),
            },
        );
        let delivered = hub.broadcast(&canonical_cdc_event());
        assert_eq!(delivered, 1);
        assert_eq!(mailbox.len(), 1);
        assert!(hub.metrics().delivered == 1);
    }

    #[test]
    fn broadcast_filters_cross_tenant() {
        let hub = RealtimeHub::new();
        hub.subscribe(
            "conn-b".to_string(),
            "user-b".to_string(),
            "tenant-b".to_string(),
            "realtime:public:orders".to_string(),
            SubscriptionFilter {
                schema: "public".to_string(),
                table: "orders".to_string(),
                op: None,
                equals: HashMap::new(),
            },
        );
        let delivered = hub.broadcast(&canonical_cdc_event());
        assert_eq!(delivered, 0);
        assert!(hub.metrics().filtered >= 1);
    }

    #[test]
    fn filter_equals_only_lets_matching_values_through() {
        let mut equals = HashMap::new();
        equals.insert("status".to_string(), "paid".to_string());
        let hub = RealtimeHub::new();
        let (_sub, _mb_paid) = hub.subscribe(
            "conn-paid".to_string(),
            "user-paid".to_string(),
            "tenant-a".to_string(),
            "realtime:public:orders".to_string(),
            SubscriptionFilter {
                schema: "public".to_string(),
                table: "orders".to_string(),
                op: None,
                equals: equals.clone(),
            },
        );
        let mut rejected_equals = HashMap::new();
        rejected_equals.insert("status".to_string(), "refunded".to_string());
        let (_sub, mb_refunded) = hub.subscribe(
            "conn-ref".to_string(),
            "user-ref".to_string(),
            "tenant-a".to_string(),
            "realtime:public:orders".to_string(),
            SubscriptionFilter {
                schema: "public".to_string(),
                table: "orders".to_string(),
                op: None,
                equals: rejected_equals,
            },
        );
        let delivered = hub.broadcast(&canonical_cdc_event());
        assert_eq!(delivered, 1);
        assert!(mb_refunded.is_empty());
    }

    #[test]
    fn presence_diff_records_joins_and_leaves() {
        let hub = RealtimeHub::new();
        let topic = "realtime:public:orders";
        let join_diff = hub.presence_join(
            topic,
            "user-a".to_string(),
            "conn-a".to_string(),
            json!({"role": "operator"}),
            "2026-05-19T12:00:00Z".to_string(),
        );
        assert!(join_diff["joins"]["user-a"]["metas"][0]["online_at"]
            .as_str()
            .unwrap()
            .starts_with("2026-"));
        assert_eq!(hub.presence_user_count(topic), 1);
        let leave_diff = hub.presence_leave(topic, "conn-a");
        assert!(leave_diff["leaves"]["user-a"]["metas"][0]["phx_ref"] == "conn-a");
        assert_eq!(hub.presence_user_count(topic), 0);
        assert_eq!(hub.metrics().presence_joins, 1);
        assert_eq!(hub.metrics().presence_leaves, 1);
    }
}
