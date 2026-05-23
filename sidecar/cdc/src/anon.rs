//! Client-side PII anonymization applied to CDC events before they reach
//! downstream sinks.
//!
//! The bundled `anon` extension can rewrite column values server-side before
//! the WAL records ever reach the sidecar. This module is a defense-in-depth
//! layer: the operator can register additional rules here for columns the
//! server-side policy does not cover, and the runtime applies them in-process
//! before the event is encoded for any sink.

// FEATURE: C3

use crate::{AnonymizationRule, AnonymizationStrategy, CdcEventEnvelope};

/// Apply every anonymization rule that matches the event in-place. Returns
/// the list of columns that were rewritten.
pub fn apply_anonymization(
    rules: &[AnonymizationRule],
    event: &mut CdcEventEnvelope,
) -> Vec<String> {
    let mut applied = Vec::new();
    for rule in rules {
        if rule.schema != event.schema || rule.table != event.table {
            continue;
        }
        for column in event.columns.iter_mut() {
            if column.name != rule.column {
                continue;
            }
            match rule.strategy {
                AnonymizationStrategy::Hash => {
                    column.value = Some(hash_value(column.value.as_deref().unwrap_or("")));
                }
                AnonymizationStrategy::Null => column.value = None,
                AnonymizationStrategy::Redact => {
                    column.value = Some("[REDACTED]".to_string());
                }
            }
            applied.push(rule.column.clone());
        }
    }
    applied
}

/// FNV-1a 64-bit hash rendered as lowercase hex. Stable, dependency-free,
/// good enough for de-identification in CDC payloads (collisions are not a
/// concern: the consumer cannot reverse the hash to recover the source).
pub fn hash_value(value: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in value.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("anon_{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CdcColumnValue, CdcOperation};

    #[test]
    fn hash_strategy_rewrites_matching_column() {
        let mut event = sample_event();
        let applied = apply_anonymization(&[rule(AnonymizationStrategy::Hash)], &mut event);
        assert_eq!(applied, vec!["email".to_string()]);
        let column = event
            .columns
            .iter()
            .find(|c| c.name == "email")
            .expect("email column");
        assert!(column.value.as_deref().unwrap().starts_with("anon_"));
        assert_ne!(column.value.as_deref().unwrap(), "person@example.com");
    }

    #[test]
    fn null_strategy_nulls_value() {
        let mut event = sample_event();
        apply_anonymization(&[rule(AnonymizationStrategy::Null)], &mut event);
        let column = event.columns.iter().find(|c| c.name == "email").unwrap();
        assert!(column.value.is_none());
    }

    #[test]
    fn redact_strategy_replaces_value() {
        let mut event = sample_event();
        apply_anonymization(&[rule(AnonymizationStrategy::Redact)], &mut event);
        let column = event.columns.iter().find(|c| c.name == "email").unwrap();
        assert_eq!(column.value.as_deref(), Some("[REDACTED]"));
    }

    #[test]
    fn non_matching_rule_leaves_event_untouched() {
        let mut event = sample_event();
        let original = event.clone();
        let applied = apply_anonymization(
            &[AnonymizationRule {
                schema: "public".to_string(),
                table: "different_table".to_string(),
                column: "email".to_string(),
                strategy: AnonymizationStrategy::Null,
            }],
            &mut event,
        );
        assert!(applied.is_empty());
        assert_eq!(event, original);
    }

    #[test]
    fn hash_is_deterministic_for_same_input() {
        assert_eq!(hash_value("foo"), hash_value("foo"));
        assert_ne!(hash_value("foo"), hash_value("bar"));
    }

    fn rule(strategy: AnonymizationStrategy) -> AnonymizationRule {
        AnonymizationRule {
            schema: "public".to_string(),
            table: "orders".to_string(),
            column: "email".to_string(),
            strategy,
        }
    }

    fn sample_event() -> CdcEventEnvelope {
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
                    name: "email".to_string(),
                    value: Some("person@example.com".to_string()),
                },
            ],
        }
    }
}
