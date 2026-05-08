//! Domain-level object for a webhook task.

use std::str::FromStr;

use chrono::Utc;
use derive_more::{Display, From};
use thiserror::Error;
use url::Url;
use uuid::Uuid;

#[derive(Debug)]
pub struct WebhookTask {
    /// Unique ID for the webhook task.
    pub id: WebhookTaskId,

    /// The scheduled deadline at which to execute the webhook task.
    pub deadline: WebhookTaskDeadline,

    /// The destination URL to send the webhook body at the deadline.
    pub url: WebhookTaskUrl,

    /// The webhook body to send to the url at the deadline.
    pub body: WebhookTaskBody,

    /// The timestamp at which the webhook task was executed.
    pub executed_at: Option<WebhookTaskExecutedAt>,
}

impl WebhookTask {
    pub fn new(
        id: WebhookTaskId,
        deadline: WebhookTaskDeadline,
        url: WebhookTaskUrl,
        body: WebhookTaskBody,
    ) -> Self {
        Self {
            id,
            deadline,
            url,
            body,
            executed_at: None,
        }
    }

    pub fn id(&self) -> &WebhookTaskId {
        &self.id
    }

    pub fn deadline(&self) -> &WebhookTaskDeadline {
        &self.deadline
    }

    pub fn url(&self) -> &WebhookTaskUrl {
        &self.url
    }

    pub fn body(&self) -> &WebhookTaskBody {
        &self.body
    }
}

// --- Domain-level data types

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, From, sqlx::Type)]
#[sqlx(transparent)]
pub struct WebhookTaskId(#[from] Uuid);
impl WebhookTaskId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}
impl FromStr for WebhookTaskId {
    type Err = <Uuid as FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = s.parse::<Uuid>()?;
        Ok(Self(uuid))
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, From, sqlx::Type)]
#[sqlx(transparent)]
pub struct WebhookTaskDeadline(#[from] chrono::DateTime<Utc>);

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, From, sqlx::Type)]
#[sqlx(transparent)]
pub struct WebhookTaskUrl(#[from] Url);
impl WebhookTaskUrl {
    pub fn url(&self) -> &Url {
        &self.0
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, From, sqlx::Type)]
#[sqlx(transparent)]
pub struct WebhookTaskBody(#[from] String);
impl WebhookTaskBody {
    pub fn into_body(self) -> String {
        self.0
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, From, sqlx::Type)]
#[sqlx(transparent)]
pub struct WebhookTaskExecutedAt(#[from] chrono::DateTime<Utc>);

// - Domain-level (programmatic) request/response objects

#[derive(Debug, Clone)]
pub struct CreateWebhookTaskRequest {
    /// The scheduled deadline at which to execute the webhook task.
    pub deadline: WebhookTaskDeadline,

    /// The destination URL to send the webhook body at the deadline.
    pub url: WebhookTaskUrl,

    /// The webhook body to send to the url at the deadline.
    pub body: WebhookTaskBody,
}

#[derive(Debug, Error)]
pub enum CreateWebhookTaskError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

/// Indicates that the given WebhookTask has been successfully executed.
#[derive(Debug, Clone)]
pub struct FinishWebhookTaskRequest {
    pub id: WebhookTaskId,
}
