// Auto-generated Test Schema
// Generated from: test-api.ttl
// Query: test-entities.rq
// Template: test-schema.tera

use serde::{Deserialize, Serialize};

// =============================================================================
// ENTITIES
// =============================================================================

/// User entity for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: String,
    pub name: String,
    pub email: String,
}

impl User {
    pub fn new(
        user_id: String,
        name: String,
        email: String,
    ) -> Self {
        Self {
            user_id,
            name,
            email,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        // Validation logic would go here
        Ok(())
    }
}

/// Product entity for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub product_id: String,
    pub name: String,
    pub price: Decimal,
}

impl Product {
    pub fn new(
        product_id: String,
        name: String,
        price: Decimal,
    ) -> Self {
        Self {
            product_id,
            name,
            price,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        // Validation logic would go here
        Ok(())
    }
}

// =============================================================================
// VALUE OBJECTS
// =============================================================================

/// Email address value object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Email {
    pub address: String,
}

// =============================================================================
// COMMANDS
// =============================================================================

/// Create a new user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
}
