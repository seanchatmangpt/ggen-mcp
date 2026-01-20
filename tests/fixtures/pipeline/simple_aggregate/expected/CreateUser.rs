use serde::{Deserialize, Serialize};

/// CreateUser command
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateUser {
    pub email: String,
    pub name: String,
}

impl CreateUser {
    /// Create a new CreateUser command
    pub fn new(email: String, name: String) -> Self {
        Self { email, name }
    }

    /// Validate the command
    pub fn validate(&self) -> Result<(), String> {
        if self.email.is_empty() {
            return Err("Email cannot be empty".to_string());
        }

        if self.name.is_empty() {
            return Err("Name cannot be empty".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user_command() {
        let cmd = CreateUser::new(
            "test@example.com".to_string(),
            "Test User".to_string(),
        );

        assert_eq!(cmd.email, "test@example.com");
        assert_eq!(cmd.name, "Test User");
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_create_user_validation_empty_email() {
        let cmd = CreateUser::new("".to_string(), "Test User".to_string());
        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_create_user_validation_empty_name() {
        let cmd = CreateUser::new("test@example.com".to_string(), "".to_string());
        assert!(cmd.validate().is_err());
    }
}
