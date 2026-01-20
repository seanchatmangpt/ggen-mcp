// Expected generated code for User aggregate

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// User aggregate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    /// Unique user identifier
    pub user_id: String,

    /// Username (unique)
    pub username: String,

    /// Email address (unique, validated)
    pub email: String,

    /// Full name (optional)
    pub full_name: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl User {
    /// Create a new user
    pub fn new(username: String, email: String) -> Self {
        Self {
            user_id: uuid::Uuid::new_v4().to_string(),
            username,
            email,
            full_name: None,
            created_at: Utc::now(),
        }
    }

    /// Set full name
    pub fn with_full_name(mut self, full_name: String) -> Self {
        self.full_name = Some(full_name);
        self
    }
}

/// User creation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCreatedEvent {
    pub user_id: String,
    pub username: String,
    pub timestamp: DateTime<Utc>,
}

impl From<&User> for UserCreatedEvent {
    fn from(user: &User) -> Self {
        Self {
            user_id: user.user_id.clone(),
            username: user.username.clone(),
            timestamp: user.created_at,
        }
    }
}
