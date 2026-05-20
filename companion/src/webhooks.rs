// FEATURE: WH2

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WebhookRegistrationPlan {
    pub name: String,
    pub table: String,
    pub events: Vec<WebhookEvent>,
    pub url: String,
    pub headers: Vec<WebhookHeader>,
    pub queue_name: String,
    pub max_retries: u32,
}

impl WebhookRegistrationPlan {
    pub fn validate(&self) -> Result<(), WebhookError> {
        validate_required("name", &self.name)?;
        validate_required("table", &self.table)?;
        validate_required("url", &self.url)?;
        validate_required("queue_name", &self.queue_name)?;
        if self.events.is_empty() {
            return Err(WebhookError::MissingRequiredField("events"));
        }
        if !self.url.starts_with("https://") && !self.url.starts_with("http://") {
            return Err(WebhookError::InvalidUrl);
        }
        for header in &self.headers {
            header.validate()?;
        }
        if self.max_retries == 0 {
            return Err(WebhookError::InvalidRetryPolicy);
        }
        Ok(())
    }

    pub fn to_sql_plan(&self) -> Result<WebhookSqlPlan, WebhookError> {
        self.validate()?;
        let events = self
            .events
            .iter()
            .map(|event| event.as_sql().to_string())
            .collect::<Vec<_>>();
        WebhookSqlPlan::new(
            "WH2",
            vec![
                format!(
                    "SELECT companion_internal.webhook_register({}, {}, {}, {}, {});",
                    sql_literal(&self.name),
                    sql_literal(&self.table),
                    sql_literal(&self.url),
                    sql_literal(&headers_json(&self.headers)),
                    self.max_retries
                ),
                format!(
                    "SELECT companion_internal.install_webhook_trigger({}, {}, {}, {});",
                    sql_literal(&self.table),
                    array_literal(&events),
                    sql_literal(&self.queue_name),
                    sql_literal(&self.name)
                ),
            ],
        )
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum WebhookEvent {
    Insert,
    Update,
    Delete,
}

impl WebhookEvent {
    fn as_sql(self) -> &'static str {
        match self {
            Self::Insert => "INSERT",
            Self::Update => "UPDATE",
            Self::Delete => "DELETE",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WebhookHeader {
    pub name: String,
    pub value_secret_ref: String,
}

impl WebhookHeader {
    fn validate(&self) -> Result<(), WebhookError> {
        validate_required("header.name", &self.name)?;
        validate_required("header.value_secret_ref", &self.value_secret_ref)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WebhookSqlPlan {
    pub feature_id: &'static str,
    pub commands: Vec<String>,
}

impl WebhookSqlPlan {
    fn new(feature_id: &'static str, commands: Vec<String>) -> Result<Self, WebhookError> {
        if commands.is_empty() || commands.iter().any(|command| command.trim().is_empty()) {
            return Err(WebhookError::MissingRequiredField("commands"));
        }
        Ok(Self {
            feature_id,
            commands,
        })
    }

    pub fn script(&self) -> String {
        self.commands.join("\n")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WebhookError {
    InvalidRetryPolicy,
    InvalidUrl,
    MissingRequiredField(&'static str),
}

impl fmt::Display for WebhookError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRetryPolicy => write!(formatter, "max_retries must be greater than zero"),
            Self::InvalidUrl => write!(formatter, "url must be http or https"),
            Self::MissingRequiredField(field) => write!(formatter, "{field} must not be empty"),
        }
    }
}

impl Error for WebhookError {}

fn validate_required(field: &'static str, value: &str) -> Result<(), WebhookError> {
    if value.trim().is_empty() {
        return Err(WebhookError::MissingRequiredField(field));
    }
    Ok(())
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn array_literal(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| sql_literal(value))
        .collect::<Vec<_>>()
        .join(", ");
    format!("ARRAY[{values}]")
}

fn headers_json(headers: &[WebhookHeader]) -> String {
    let headers = headers
        .iter()
        .map(|header| {
            format!(
                "\"{}\":\"{}\"",
                json_escape(&header.name),
                json_escape(&header.value_secret_ref)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{headers}}}")
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_registration_renders_trigger_install() {
        let plan = WebhookRegistrationPlan {
            name: "orders-webhook".to_string(),
            table: "public.orders".to_string(),
            events: vec![WebhookEvent::Insert, WebhookEvent::Update],
            url: "https://hooks.example.test/orders".to_string(),
            headers: vec![WebhookHeader {
                name: "Authorization".to_string(),
                value_secret_ref: "secret://webhooks/orders".to_string(),
            }],
            queue_name: "companion.webhook_queue".to_string(),
            max_retries: 8,
        }
        .to_sql_plan()
        .unwrap();

        assert_eq!(plan.feature_id, "WH2");
        assert!(plan.script().contains("webhook_register"));
        assert!(plan.script().contains("install_webhook_trigger"));
    }

    #[test]
    fn webhook_requires_http_url() {
        let plan = WebhookRegistrationPlan {
            name: "orders-webhook".to_string(),
            table: "public.orders".to_string(),
            events: vec![WebhookEvent::Delete],
            url: "secret://orders".to_string(),
            headers: Vec::new(),
            queue_name: "companion.webhook_queue".to_string(),
            max_retries: 3,
        };

        assert_eq!(plan.validate(), Err(WebhookError::InvalidUrl));
    }
}
