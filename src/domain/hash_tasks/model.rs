//! Domain-level object for a hash task.

use std::str::FromStr;

use chrono::Utc;
use derive_more::{Display, From};
use thiserror::Error;
use uuid::Uuid;

pub struct HashTask {
    /// Unique ID for the hash task.
    pub id: HashTaskId,

    /// The scheduled deadline at which to execute the hash task.
    pub deadline: HashTaskDeadline,

    /// The input data to hash.
    pub secret: HashTaskSecret,

    /// The timestamp at which the hash task was executed.
    pub executed_at: Option<HashTaskExecutedAt>,
}

impl HashTask {
    pub fn new(id: HashTaskId, deadline: HashTaskDeadline, secret: HashTaskSecret) -> Self {
        Self {
            id,
            deadline,
            secret,
            executed_at: None,
        }
    }

    pub fn id(&self) -> &HashTaskId {
        &self.id
    }

    pub fn deadline(&self) -> &HashTaskDeadline {
        &self.deadline
    }

    pub fn secret(&self) -> &HashTaskSecret {
        &self.secret
    }
}

// --- Domain-level data types

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, From, sqlx::Type)]
#[sqlx(transparent)]
pub struct HashTaskId(#[from] Uuid);
impl HashTaskId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}
impl FromStr for HashTaskId {
    type Err = <Uuid as FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = s.parse::<Uuid>()?;
        Ok(Self(uuid))
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, From, sqlx::Type)]
#[sqlx(transparent)]
pub struct HashTaskDeadline(#[from] chrono::DateTime<Utc>);

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, From, sqlx::Type)]
#[sqlx(transparent)]
pub struct HashTaskSecret(#[from] String);
impl HashTaskSecret {
    pub fn data(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash, From, sqlx::Type)]
#[sqlx(transparent)]
pub struct HashTaskExecutedAt(#[from] chrono::DateTime<Utc>);

// - Domain-level (programmatic) request/response objects

#[derive(Debug, Clone)]
pub struct CreateHashTaskRequest {
    /// The scheduled deadline at which to execute the hash task.
    pub deadline: HashTaskDeadline,

    /// The input data to hash.
    pub secret: HashTaskSecret,
}

#[derive(Debug, Error)]
pub enum CreateHashTaskError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}
