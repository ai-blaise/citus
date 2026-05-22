// FEATURE: WH1

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WebhookSpec {
    pub table: String,
    pub events: Vec<WebhookEvent>,
    pub url: String,
    pub headers_secret_ref: Option<String>,
    pub retry_policy: WebhookRetryPolicy,
    pub payload_template: Option<String>,
}

impl WebhookSpec {
    pub fn validate(&self) -> Result<(), WebhookSpecError> {
        validate_required("table", &self.table)?;
        validate_required("url", &self.url)?;
        if !self.url.starts_with("https://") && !self.url.starts_with("http://") {
            return Err(WebhookSpecError::InvalidUrl);
        }
        if self.events.is_empty() {
            return Err(WebhookSpecError::MissingRequiredField("events"));
        }
        validate_optional("headers_secret_ref", &self.headers_secret_ref)?;
        validate_optional("payload_template", &self.payload_template)?;
        self.retry_policy.validate()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum WebhookEvent {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WebhookRetryPolicy {
    pub max_attempts: u32,
    pub backoff: String,
    pub dead_letter_table: Option<String>,
}

impl WebhookRetryPolicy {
    fn validate(&self) -> Result<(), WebhookSpecError> {
        if self.max_attempts == 0 {
            return Err(WebhookSpecError::InvalidRetryAttempts);
        }
        validate_required("retry_policy.backoff", &self.backoff)?;
        validate_optional("retry_policy.dead_letter_table", &self.dead_letter_table)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WebhookSpecError {
    InvalidRetryAttempts,
    InvalidUrl,
    MissingRequiredField(&'static str),
}

impl fmt::Display for WebhookSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRetryAttempts => {
                write!(formatter, "max_attempts must be greater than zero")
            }
            Self::InvalidUrl => write!(formatter, "url must start with http:// or https://"),
            Self::MissingRequiredField(field) => {
                write!(formatter, "{field} must not be empty")
            }
        }
    }
}

impl Error for WebhookSpecError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), WebhookSpecError> {
    if value.trim().is_empty() {
        return Err(WebhookSpecError::MissingRequiredField(field));
    }
    Ok(())
}

fn validate_optional(field: &'static str, value: &Option<String>) -> Result<(), WebhookSpecError> {
    if matches!(value, Some(value) if value.trim().is_empty()) {
        return Err(WebhookSpecError::MissingRequiredField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_webhook_passes() {
        let spec = WebhookSpec {
            table: "public.orders".to_string(),
            events: vec![WebhookEvent::Insert, WebhookEvent::Update],
            url: "https://example.com/orders".to_string(),
            headers_secret_ref: Some("orders-webhook".to_string()),
            retry_policy: WebhookRetryPolicy {
                max_attempts: 5,
                backoff: "exponential:1s:30s".to_string(),
                dead_letter_table: Some("webhook_dead_letters".to_string()),
            },
            payload_template: Some("{\"table\":\"orders\"}".to_string()),
        };

        assert_eq!(spec.validate(), Ok(()));
    }

    #[test]
    fn webhook_rejects_invalid_url() {
        let mut spec = minimal_spec();
        spec.url = "ftp://example.com".to_string();

        assert_eq!(spec.validate(), Err(WebhookSpecError::InvalidUrl));
    }

    #[test]
    fn webhook_requires_event_list() {
        let mut spec = minimal_spec();
        spec.events = Vec::new();

        assert_eq!(
            spec.validate(),
            Err(WebhookSpecError::MissingRequiredField("events"))
        );
    }

    fn minimal_spec() -> WebhookSpec {
        WebhookSpec {
            table: "public.orders".to_string(),
            events: vec![WebhookEvent::Insert],
            url: "https://example.com/orders".to_string(),
            headers_secret_ref: None,
            retry_policy: WebhookRetryPolicy {
                max_attempts: 3,
                backoff: "fixed:5s".to_string(),
                dead_letter_table: None,
            },
            payload_template: None,
        }
    }
}
