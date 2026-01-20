use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User aggregate root
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
}

impl User {
    /// Create a new User
    pub fn new(email: String, name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            email,
            name,
        }
    }

    /// Get the aggregate ID
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Update email
    pub fn update_email(&mut self, email: String) {
        self.email = email;
    }

    /// Update name
    pub fn update_name(&mut self, name: String) {
        self.name = name;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new(
            "test@example.com".to_string(),
            "Test User".to_string(),
        );

        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.name, "Test User");
    }

    #[test]
    fn test_user_update_email() {
        let mut user = User::new(
            "old@example.com".to_string(),
            "Test User".to_string(),
        );

        user.update_email("new@example.com".to_string());
        assert_eq!(user.email, "new@example.com");
    }
}
