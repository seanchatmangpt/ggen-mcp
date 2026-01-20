//! Ontology Generation Example
//!
//! Demonstrates the full workflow for generating code from RDF ontologies:
//! 1. Define schema (Zod or JSON Schema)
//! 2. Validate schema
//! 3. Generate entity code
//! 4. Validate generated code
//! 5. Use generated code
//!
//! Run with: `cargo run --example ontology_generation_example`

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Example parameters for schema-based generation
#[derive(Debug, Serialize)]
struct GenerateFromSchemaParams {
    schema_type: String,
    schema_content: String,
    entity_name: String,
    features: Vec<String>,
    output_path: PathBuf,
}

/// Example response from generation
#[derive(Debug, Deserialize)]
struct GenerateFromSchemaResponse {
    entity_name: String,
    output_path: PathBuf,
    generated_code: String,
    features_applied: Vec<String>,
    statistics: GenerationStatistics,
}

#[derive(Debug, Deserialize)]
struct GenerationStatistics {
    fields_generated: usize,
    lines_of_code: usize,
    validation_rules: usize,
}

/// Example: Full ontology generation workflow
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Ontology Generation Example ===\n");

    // Step 1: Define Zod schema
    println!("Step 1: Define Zod schema");
    let user_schema = r#"z.object({
  id: z.string().uuid(),
  email: z.string().email(),
  username: z.string().min(3).max(20),
  age: z.number().int().min(18).max(120),
  created_at: z.string().datetime(),
  is_active: z.boolean().default(true)
})"#;
    println!("Schema:\n{}\n", user_schema);

    // Step 2: Validate schema (simulated)
    println!("Step 2: Validate schema");
    validate_schema(user_schema).await?;
    println!("✓ Schema validation passed\n");

    // Step 3: Generate entity code
    println!("Step 3: Generate entity code");
    let params = GenerateFromSchemaParams {
        schema_type: "zod".to_string(),
        schema_content: user_schema.to_string(),
        entity_name: "User".to_string(),
        features: vec![
            "serde".to_string(),
            "validation".to_string(),
            "builder".to_string(),
        ],
        output_path: PathBuf::from("src/generated/user.rs"),
    };

    let response = generate_from_schema(params).await?;
    println!("✓ Generated {} lines of code", response.statistics.lines_of_code);
    println!("✓ {} fields generated", response.statistics.fields_generated);
    println!("✓ {} validation rules", response.statistics.validation_rules);
    println!("✓ Features: {}\n", response.features_applied.join(", "));

    // Step 4: Preview generated code
    println!("Step 4: Preview generated code");
    println!("Output path: {}\n", response.output_path.display());
    println!("Generated code (preview):\n{}", &response.generated_code[..500.min(response.generated_code.len())]);
    println!("... (truncated)\n");

    // Step 5: Validate generated code
    println!("Step 5: Validate generated code");
    validate_generated_code(&response.generated_code).await?;
    println!("✓ Generated code validation passed\n");

    // Step 6: Demonstrate usage of generated code (simulated)
    println!("Step 6: Example usage of generated code");
    demonstrate_usage();
    println!();

    // Step 7: Full ontology sync workflow (simulated)
    println!("Step 7: Full ontology sync workflow");
    demonstrate_full_sync().await?;

    println!("=== Example Complete ===");
    Ok(())
}

/// Simulate schema validation
async fn validate_schema(schema: &str) -> Result<()> {
    // In real implementation, this would call validate_ontology tool
    println!("  Validating schema syntax...");

    // Check for basic syntax
    if !schema.contains("z.object") {
        anyhow::bail!("Invalid Zod schema: missing z.object");
    }

    println!("  ✓ Syntax valid");
    println!("  ✓ All fields have types");
    println!("  ✓ Constraints valid");

    Ok(())
}

/// Simulate code generation from schema
async fn generate_from_schema(params: GenerateFromSchemaParams) -> Result<GenerateFromSchemaResponse> {
    println!("  Parsing {} schema...", params.schema_type);
    println!("  Extracting fields and constraints...");
    println!("  Rendering template with features: {:?}", params.features);
    println!("  Formatting with rustfmt...");

    // Simulated generated code
    let generated_code = format!(r#"use chrono::{{DateTime, Utc}};
use serde::{{Deserialize, Serialize}};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {} {{
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub age: u8,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}}

impl {} {{
    pub fn validate(&self) -> Result<(), ValidationError> {{
        validate_email(&self.email)?;

        if self.username.len() < 3 || self.username.len() > 20 {{
            return Err(ValidationError::InvalidUsernameLength);
        }}

        if self.age < 18 || self.age > 120 {{
            return Err(ValidationError::InvalidAge);
        }}

        Ok(())
    }}
}}

#[derive(Default)]
pub struct {}Builder {{
    id: Option<Uuid>,
    email: Option<String>,
    username: Option<String>,
    age: Option<u8>,
    created_at: Option<DateTime<Utc>>,
    is_active: Option<bool>,
}}

impl {}Builder {{
    pub fn new() -> Self {{
        Self::default()
    }}

    pub fn id(mut self, id: Uuid) -> Self {{
        self.id = Some(id);
        self
    }}

    pub fn email(mut self, email: impl Into<String>) -> Self {{
        self.email = Some(email.into());
        self
    }}

    pub fn username(mut self, username: impl Into<String>) -> Self {{
        self.username = Some(username.into());
        self
    }}

    pub fn age(mut self, age: u8) -> Self {{
        self.age = Some(age);
        self
    }}

    pub fn created_at(mut self, created_at: DateTime<Utc>) -> Self {{
        self.created_at = Some(created_at);
        self
    }}

    pub fn is_active(mut self, is_active: bool) -> Self {{
        self.is_active = Some(is_active);
        self
    }}

    pub fn build(self) -> Result<{}, BuilderError> {{
        let user = {} {{
            id: self.id.ok_or(BuilderError::MissingField("id"))?,
            email: self.email.ok_or(BuilderError::MissingField("email"))?,
            username: self.username.ok_or(BuilderError::MissingField("username"))?,
            age: self.age.ok_or(BuilderError::MissingField("age"))?,
            created_at: self.created_at.unwrap_or_else(Utc::now),
            is_active: self.is_active.unwrap_or(true),
        }};

        user.validate()?;
        Ok(user)
    }}
}}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {{
    #[error("Invalid email format")]
    InvalidEmail,
    #[error("Username must be between 3 and 20 characters")]
    InvalidUsernameLength,
    #[error("Age must be between 18 and 120")]
    InvalidAge,
}}

#[derive(Debug, thiserror::Error)]
pub enum BuilderError {{
    #[error("Missing required field: {{0}}")]
    MissingField(&'static str),
    #[error("Validation failed: {{0}}")]
    Validation(#[from] ValidationError),
}}

fn validate_email(email: &str) -> Result<(), ValidationError> {{
    let email_regex = regex::Regex::new(r"^[^@]+@[^@]+\.[^@]+$").unwrap();
    if email_regex.is_match(email) {{
        Ok(())
    }} else {{
        Err(ValidationError::InvalidEmail)
    }}
}}"#,
        params.entity_name, params.entity_name, params.entity_name, params.entity_name,
        params.entity_name, params.entity_name
    );

    Ok(GenerateFromSchemaResponse {
        entity_name: params.entity_name.clone(),
        output_path: params.output_path,
        generated_code,
        features_applied: params.features,
        statistics: GenerationStatistics {
            fields_generated: 6,
            lines_of_code: 127,
            validation_rules: 3,
        },
    })
}

/// Simulate validation of generated code
async fn validate_generated_code(code: &str) -> Result<()> {
    println!("  Running quality gates...");

    // Gate 1: No TODOs
    if code.contains("TODO") || code.contains("unimplemented!") {
        anyhow::bail!("Generated code contains TODOs");
    }
    println!("  ✓ Gate 1: No TODOs");

    // Gate 2: Syntax valid (simulate)
    if !code.contains("pub struct") {
        anyhow::bail!("Generated code missing struct definition");
    }
    println!("  ✓ Gate 2: Syntax valid");

    // Gate 3: All types imported
    if code.contains("Uuid") && !code.contains("use uuid::Uuid") {
        anyhow::bail!("Missing import for Uuid");
    }
    println!("  ✓ Gate 3: All imports present");

    // Gate 4: validate() function present
    if !code.contains("pub fn validate") {
        anyhow::bail!("Missing validate() function");
    }
    println!("  ✓ Gate 4: validate() implemented");

    Ok(())
}

/// Demonstrate usage of generated code
fn demonstrate_usage() {
    println!(r#"  Example usage:

  use crate::generated::user::{{User, UserBuilder}};
  use uuid::Uuid;

  // Using builder pattern
  let user = UserBuilder::new()
      .id(Uuid::new_v4())
      .email("alice@example.com")
      .username("alice123")
      .age(25)
      .is_active(true)
      .build()?;

  // Validation is automatic
  assert!(user.validate().is_ok());

  // Serialize to JSON
  let json = serde_json::to_string_pretty(&user)?;
  println!("{{}}", json);

  Output:
  {{
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "alice@example.com",
    "username": "alice123",
    "age": 25,
    "created_at": "2026-01-20T10:23:45Z",
    "is_active": true
  }}"#);
}

/// Demonstrate full ontology sync workflow (13 steps)
async fn demonstrate_full_sync() -> Result<()> {
    println!("  Simulating full ontology sync pipeline...\n");

    let stages = vec![
        ("1. Load Ontology", 234, "Loaded 347 triples"),
        ("2. Validate SHACL", 89, "All constraints satisfied"),
        ("3. Resolve Dependencies", 12, "2 imports resolved"),
        ("4. Execute SPARQL Queries", 456, "5 tools extracted"),
        ("5. Validate Query Results", 23, "All results valid"),
        ("6. Render Tera Templates", 789, "1 template rendered"),
        ("7. Validate Generated Code", 145, "Syntax valid"),
        ("8. Format with rustfmt", 312, "Formatted 1 file"),
        ("9. Check Compilation", 3421, "cargo check passed"),
        ("10. Detect TODOs", 67, "0 TODOs found"),
        ("11. Run Tests", 2134, "All tests passed"),
        ("12. Generate Audit Receipt", 45, "Receipt generated"),
        ("13. Write Files", 189, "1 file written"),
    ];

    let mut total_duration = 0u64;

    for (stage, duration, details) in stages {
        total_duration += duration;
        println!("  ✓ {} ({}ms) - {}", stage, duration, details);
        // Simulate processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    println!("\n  Pipeline completed successfully!");
    println!("  Total duration: {}ms (~{}s)", total_duration, total_duration / 1000);
    println!("  Audit receipt: .ggen/receipts/receipt-20260120-102345-a7b9c1d2.json");

    Ok(())
}

// Helper types for demonstration
fn validate_email(_email: &str) -> Result<(), ValidationError> {
    Ok(())
}

#[derive(Debug)]
enum ValidationError {
    InvalidEmail,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidEmail => write!(f, "Invalid email format"),
        }
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validate_schema() {
        let schema = r#"z.object({ id: z.string().uuid() })"#;
        assert!(validate_schema(schema).await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_generated_code() {
        let code = r#"
            use uuid::Uuid;
            pub struct User { pub id: Uuid }
            impl User {
                pub fn validate(&self) -> Result<(), ()> { Ok(()) }
            }
        "#;
        assert!(validate_generated_code(code).await.is_ok());
    }

    #[tokio::test]
    async fn test_reject_code_with_todos() {
        let code = "pub struct User { /* TODO */ }";
        assert!(validate_generated_code(code).await.is_err());
    }

    #[tokio::test]
    async fn test_reject_code_without_validate() {
        let code = "pub struct User { pub id: String }";
        assert!(validate_generated_code(code).await.is_err());
    }
}
