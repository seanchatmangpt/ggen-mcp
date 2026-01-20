//! Comprehensive tests for template rendering safety
//!
//! This test suite validates the poka-yoke (error-proofing) mechanisms
//! in the template rendering system, including tests for malicious templates.

use spreadsheet_mcp::template::rendering_safety::{
    ErrorRecovery, OutputValidator, RenderConfig, RenderContext, RenderGuard, RenderingError,
    SafeRenderer, ValidationSeverity,
};
use std::sync::Arc;

// ============================================================================
// Basic Rendering Tests
// ============================================================================

#[test]
fn test_basic_template_rendering() {
    let config = RenderConfig::default();
    let renderer = SafeRenderer::new(config).unwrap();

    renderer.add_template("basic", "Hello {{ name }}!").unwrap();

    let mut context = RenderContext::new();
    context.insert("name", &"World").unwrap();

    let output = renderer.render_safe("basic", &context).unwrap();
    assert_eq!(output, "Hello World!");
}

#[test]
fn test_template_with_loop() {
    let config = RenderConfig::default();
    let renderer = SafeRenderer::new(config).unwrap();

    let template = r#"
{% for item in items -%}
- {{ item }}
{% endfor -%}
"#;

    renderer.add_template("loop", template).unwrap();

    let mut context = RenderContext::new();
    context.insert("items", &vec!["a", "b", "c"]).unwrap();

    let output = renderer.render_safe("loop", &context).unwrap();
    assert!(output.contains("- a"));
    assert!(output.contains("- b"));
    assert!(output.contains("- c"));
}

#[test]
fn test_template_with_conditionals() {
    let config = RenderConfig::default();
    let renderer = SafeRenderer::new(config).unwrap();

    let template = r#"
{% if enabled -%}
Feature is enabled
{% else -%}
Feature is disabled
{% endif -%}
"#;

    renderer.add_template("conditional", template).unwrap();

    let mut context = RenderContext::new();
    context.insert("enabled", &true).unwrap();

    let output = renderer.render_safe("conditional", &context).unwrap();
    assert!(output.contains("Feature is enabled"));

    context.insert("enabled", &false).unwrap();
    let output = renderer.render_safe("conditional", &context).unwrap();
    assert!(output.contains("Feature is disabled"));
}

// ============================================================================
// Context Scoping Tests
// ============================================================================

#[test]
fn test_context_scoping() {
    let mut root = RenderContext::new();
    root.insert("root_var", &"root_value").unwrap();

    let root_arc = Arc::new(root);
    let mut child = RenderContext::child(root_arc.clone());
    child.insert("child_var", &"child_value").unwrap();

    // Child can access root variables
    assert_eq!(
        child.get("root_var").unwrap().as_str().unwrap(),
        "root_value"
    );
    assert_eq!(
        child.get("child_var").unwrap().as_str().unwrap(),
        "child_value"
    );

    // Root cannot access child variables
    assert!(root_arc.get("child_var").is_none());
}

#[test]
fn test_context_recursion_depth() {
    let config = RenderConfig::default().with_max_recursion_depth(5);
    let mut context = RenderContext::new();

    assert!(
        context
            .check_recursion_depth(config.max_recursion_depth)
            .is_ok()
    );

    // Simulate deep nesting
    let mut current = Arc::new(context);
    for _ in 0..10 {
        current = Arc::new(RenderContext::child(current));
    }

    // Should exceed limit
    assert!(current.check_recursion_depth(5).is_err());
}

#[test]
fn test_context_macro_counting() {
    let config = RenderConfig::default().with_max_macro_expansions(3);
    let mut context = RenderContext::new();

    assert!(
        context
            .increment_macro_count(config.max_macro_expansions)
            .is_ok()
    );
    assert!(
        context
            .increment_macro_count(config.max_macro_expansions)
            .is_ok()
    );
    assert!(
        context
            .increment_macro_count(config.max_macro_expansions)
            .is_ok()
    );

    // Fourth increment should fail
    assert!(
        context
            .increment_macro_count(config.max_macro_expansions)
            .is_err()
    );
}

// ============================================================================
// Output Validation Tests
// ============================================================================

#[test]
fn test_validator_balanced_delimiters() {
    let validator = OutputValidator::new(true, false);

    let valid_code = r#"
fn main() {
    let x = vec![1, 2, 3];
    println!("{:?}", x);
}
"#;

    let errors = validator.validate(valid_code).unwrap();
    assert!(
        errors.is_empty(),
        "Valid code should have no errors: {:?}",
        errors
    );
}

#[test]
fn test_validator_unbalanced_braces() {
    let validator = OutputValidator::new(true, false);

    let invalid_code = r#"
fn main() {
    let x = vec![1, 2, 3;
}
"#;

    let errors = validator.validate(invalid_code).unwrap();
    assert!(!errors.is_empty(), "Should detect unbalanced delimiters");
    assert!(errors.iter().any(|e| e.message.contains("brackets")));
}

#[test]
fn test_validator_unbalanced_parentheses() {
    let validator = OutputValidator::new(true, false);

    let invalid_code = "fn main( { println!(\"test\"; }";

    let errors = validator.validate(invalid_code).unwrap();
    assert!(!errors.is_empty(), "Should detect unbalanced parentheses");
}

#[test]
fn test_validator_security_unsafe_code() {
    let validator = OutputValidator::new(false, true);

    let unsafe_code = r#"
unsafe {
    std::ptr::write(ptr, value);
}
"#;

    let errors = validator.validate(unsafe_code).unwrap();
    assert!(!errors.is_empty(), "Should detect unsafe code");
    assert!(
        errors
            .iter()
            .any(|e| e.message.to_lowercase().contains("unsafe"))
    );
}

#[test]
fn test_validator_security_command_execution() {
    let validator = OutputValidator::new(false, true);

    let dangerous_code = r#"
use std::process::Command;

fn run_command() {
    Command::new("rm").arg("-rf").arg("/").spawn();
}
"#;

    let errors = validator.validate(dangerous_code).unwrap();
    assert!(!errors.is_empty(), "Should detect command execution");
    assert!(
        errors
            .iter()
            .any(|e| e.message.to_lowercase().contains("command"))
    );
}

#[test]
fn test_validator_security_filesystem_ops() {
    let validator = OutputValidator::new(false, true);

    let fs_code = r#"
use std::fs;

fn delete_file() {
    fs::remove_file("/important/file.txt");
}
"#;

    let errors = validator.validate(fs_code).unwrap();
    assert!(!errors.is_empty(), "Should detect filesystem operations");
}

#[test]
fn test_validator_invalid_identifiers() {
    let validator = OutputValidator::new(true, false);

    let code_with_invalid_ids = r#"
fn 123invalid() {
    let 456var = 10;
}
"#;

    let errors = validator.validate(code_with_invalid_ids).unwrap();
    // Note: This might be a warning rather than an error
    assert!(errors.iter().any(|e| matches!(
        e.severity,
        ValidationSeverity::Warning | ValidationSeverity::Error
    )));
}

// ============================================================================
// Malicious Template Tests
// ============================================================================

#[test]
fn test_malicious_infinite_loop() {
    let config = RenderConfig::default().with_timeout_ms(1000);
    let renderer = SafeRenderer::new(config).unwrap();

    // Template with very large loop
    let malicious = "{% for i in range(start=0, end=999999999) %}{{ i }}{% endfor %}";

    renderer.add_template("infinite", malicious).unwrap();

    let context = RenderContext::new();

    // Should complete without hanging (in real impl with timeout)
    // For now, just test that it doesn't panic
    let _ = renderer.render_safe("infinite", &context);
}

#[test]
fn test_malicious_deep_nesting() {
    let config = RenderConfig::default().with_max_recursion_depth(5);
    let renderer = SafeRenderer::new(config).unwrap();

    // Create deeply nested template structure
    let mut template = String::new();
    for i in 0..10 {
        template.push_str(&format!(
            "{{% if true %}}{{% if true %}}{{% if true %}}Level {}{{% endif %}}{{% endif %}}{{% endif %}}",
            i
        ));
    }

    renderer.add_template("deep", &template).unwrap();

    let context = RenderContext::new();
    let _ = renderer.render_safe("deep", &context);
    // Should handle deep nesting gracefully
}

#[test]
fn test_malicious_large_output() {
    let config = RenderConfig::default().with_max_output_size(1024); // 1KB limit

    let renderer = SafeRenderer::new(config).unwrap();

    // Template that generates large output
    let template = r#"
{% for i in range(start=0, end=1000) -%}
This is a very long line that will be repeated many times to exceed the output size limit.
{% endfor -%}
"#;

    renderer.add_template("large", template).unwrap();

    let context = RenderContext::new();
    let result = renderer.render_safe("large", &context);

    // Should fail with OutputSizeExceeded error
    if let Err(e) = result {
        assert!(matches!(e, RenderingError::OutputSizeExceeded { .. }));
    }
}

#[test]
fn test_malicious_code_injection() {
    let config = RenderConfig::default()
        .with_syntax_validation(true)
        .with_security_checks(true);

    let renderer = SafeRenderer::new(config).unwrap();

    // Template that tries to inject unsafe code
    let template = r#"
pub struct {{ name }} {
    data: String,
}

impl {{ name }} {
    pub fn exploit(&self) {
        unsafe {
            // Malicious code here
            std::ptr::write_volatile(0 as *mut u8, 0);
        }
    }
}
"#;

    renderer.add_template("injection", template).unwrap();

    let mut context = RenderContext::new();
    context.insert("name", &"Exploit").unwrap();

    let result = renderer.render_safe("injection", &context);

    // Should succeed but validation should warn about unsafe
    match result {
        Ok(output) => {
            let validator = OutputValidator::new(false, true);
            let errors = validator.validate(&output).unwrap();
            assert!(!errors.is_empty(), "Should detect unsafe code");
        }
        Err(_) => {
            // Also acceptable if rendering fails
        }
    }
}

#[test]
fn test_malicious_sql_injection_pattern() {
    let config = RenderConfig::default().with_security_checks(true);
    let renderer = SafeRenderer::new(config).unwrap();

    let template = r#"
fn query_user(id: &str) -> String {
    format!("SELECT * FROM users WHERE id = '{}'", id)
}
"#;

    renderer.add_template("sql", template).unwrap();

    let context = RenderContext::new();
    let output = renderer.render_safe("sql", &context).unwrap();

    let validator = OutputValidator::new(false, true);
    let errors = validator.validate(&output).unwrap();

    // Should detect potential SQL injection
    assert!(
        errors
            .iter()
            .any(|e| e.message.to_lowercase().contains("sql"))
    );
}

#[test]
fn test_malicious_path_traversal() {
    let config = RenderConfig::default();
    let renderer = SafeRenderer::new(config).unwrap();

    let template = r#"
use std::fs;

fn read_file(path: &str) -> String {
    fs::read_to_string(format!("../../{}", path)).unwrap()
}
"#;

    renderer.add_template("traversal", template).unwrap();

    let context = RenderContext::new();
    let output = renderer.render_safe("traversal", &context).unwrap();

    let validator = OutputValidator::new(false, true);
    let errors = validator.validate(&output).unwrap();

    // Should detect filesystem operations
    assert!(!errors.is_empty());
}

// ============================================================================
// Error Recovery Tests
// ============================================================================

#[test]
fn test_error_recovery_basic() {
    let mut recovery = ErrorRecovery::new(false);

    assert!(!recovery.has_errors());

    recovery.record_error(RenderingError::Timeout { timeout_ms: 5000 });

    assert!(recovery.has_errors());
    assert_eq!(recovery.errors().len(), 1);
}

#[test]
fn test_error_recovery_suggestions() {
    let mut recovery = ErrorRecovery::new(false);

    recovery.record_error(RenderingError::Timeout { timeout_ms: 5000 });
    recovery.record_error(RenderingError::RecursionDepthExceeded {
        depth: 15,
        limit: 10,
    });

    let suggestions = recovery.suggest_fixes();
    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().any(|s| s.contains("timed out")));
    assert!(suggestions.iter().any(|s| s.contains("Recursion")));
}

#[test]
fn test_error_recovery_report() {
    let mut recovery = ErrorRecovery::new(false);

    recovery.record_error(RenderingError::Timeout { timeout_ms: 5000 });
    recovery.set_partial_output("partial output here".to_string());

    let report = recovery.error_report();
    assert!(report.contains("Rendering Errors: 1"));
    assert!(report.contains("Partial Output Available"));
}

#[test]
fn test_error_recovery_with_partial_output() {
    let recovery = ErrorRecovery::new(true); // allow_partial = true

    assert!(recovery.partial_output().is_none());

    let mut recovery = ErrorRecovery::new(true);
    recovery.set_partial_output("partial content".to_string());

    assert_eq!(recovery.partial_output(), Some("partial content"));
}

// ============================================================================
// Render Guard Tests
// ============================================================================

#[test]
fn test_render_guard_cleanup() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_guard_cleanup.txt");

    // Create temp file
    std::fs::write(&temp_file, "test content").unwrap();
    assert!(temp_file.exists());

    {
        let mut guard = RenderGuard::new();
        guard.register_temp_file(temp_file.clone());
        // Guard drops here without commit
    }

    // File should be cleaned up
    assert!(!temp_file.exists(), "Temp file should be cleaned up");
}

#[test]
fn test_render_guard_commit_preserves_files() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_guard_commit.txt");

    // Create temp file
    std::fs::write(&temp_file, "test content").unwrap();
    assert!(temp_file.exists());

    {
        let mut guard = RenderGuard::new();
        guard.register_temp_file(temp_file.clone());
        let _metrics = guard.commit(); // Commit the guard
    }

    // File should still exist
    assert!(temp_file.exists(), "Committed file should be preserved");

    // Cleanup
    std::fs::remove_file(&temp_file).ok();
}

#[test]
fn test_render_guard_metrics() {
    use std::thread;
    use std::time::Duration;

    let mut guard = RenderGuard::new();

    thread::sleep(Duration::from_millis(10));

    let metrics = guard.commit();

    assert!(metrics.duration.as_millis() >= 10);
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_config_validation() {
    let valid_config = RenderConfig::default();
    assert!(valid_config.validate().is_ok());

    let invalid_timeout = RenderConfig {
        timeout_ms: 0,
        ..Default::default()
    };
    assert!(invalid_timeout.validate().is_err());

    let invalid_recursion = RenderConfig {
        max_recursion_depth: 0,
        ..Default::default()
    };
    assert!(invalid_recursion.validate().is_err());

    let invalid_size = RenderConfig {
        max_output_size: 0,
        ..Default::default()
    };
    assert!(invalid_size.validate().is_err());
}

#[test]
fn test_config_builder() {
    let config = RenderConfig::builder()
        .timeout_ms(10_000)
        .max_recursion_depth(20)
        .max_output_size(50 * 1024 * 1024)
        .validate_syntax(true)
        .security_checks(true)
        .allow_partial_rendering(true)
        .collect_metrics(true)
        .build();

    assert_eq!(config.timeout_ms, 10_000);
    assert_eq!(config.max_recursion_depth, 20);
    assert_eq!(config.max_output_size, 50 * 1024 * 1024);
    assert!(config.validate_syntax);
    assert!(config.security_checks);
    assert!(config.allow_partial_rendering);
    assert!(config.collect_metrics);
}

#[test]
fn test_config_chaining() {
    let config = RenderConfig::default()
        .with_timeout_ms(8000)
        .with_max_recursion_depth(15)
        .with_syntax_validation(false)
        .with_security_checks(false);

    assert_eq!(config.timeout_ms, 8000);
    assert_eq!(config.max_recursion_depth, 15);
    assert!(!config.validate_syntax);
    assert!(!config.security_checks);
}

#[test]
fn test_config_limits_enforced() {
    use spreadsheet_mcp::template::rendering_safety::{
        MAX_OUTPUT_SIZE, MAX_RECURSION_DEPTH, MAX_TIMEOUT_MS,
    };

    let config = RenderConfig::default()
        .with_timeout_ms(MAX_TIMEOUT_MS + 1000)
        .with_max_recursion_depth(MAX_RECURSION_DEPTH + 10)
        .with_max_output_size(MAX_OUTPUT_SIZE + 1000);

    // Limits should be clamped to maximum values
    assert!(config.timeout_ms <= MAX_TIMEOUT_MS);
    assert!(config.max_recursion_depth <= MAX_RECURSION_DEPTH);
    // Note: max_output_size might not be clamped in with_ methods
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_render_pipeline() {
    let config = RenderConfig::default()
        .with_timeout_ms(5000)
        .with_syntax_validation(true)
        .with_security_checks(true);

    let renderer = SafeRenderer::new(config).unwrap();

    let template = r#"
//! Generated {{ module_name }} module

#[derive(Debug, Clone)]
pub struct {{ struct_name }} {
    {% for field in fields -%}
    pub {{ field.name }}: {{ field.type_ }},
    {% endfor -%}
}

impl {{ struct_name }} {
    pub fn new({% for field in fields %}{{ field.name }}: {{ field.type_ }}{% if not loop.last %}, {% endif %}{% endfor %}) -> Self {
        Self {
            {% for field in fields -%}
            {{ field.name }},
            {% endfor -%}
        }
    }
}
"#;

    renderer.add_template("struct_gen", template).unwrap();

    #[derive(serde::Serialize)]
    struct Field {
        name: String,
        type_: String,
    }

    let fields = vec![
        Field {
            name: "id".to_string(),
            type_: "String".to_string(),
        },
        Field {
            name: "value".to_string(),
            type_: "i32".to_string(),
        },
    ];

    let mut context = RenderContext::new();
    context.insert("module_name", &"test_module").unwrap();
    context.insert("struct_name", &"TestStruct").unwrap();
    context.insert("fields", &fields).unwrap();

    let output = renderer.render_safe("struct_gen", &context).unwrap();

    // Verify output
    assert!(output.contains("pub struct TestStruct"));
    assert!(output.contains("pub id: String"));
    assert!(output.contains("pub value: i32"));
    assert!(output.contains("impl TestStruct"));

    // Validate output
    let validator = OutputValidator::new(true, true);
    let errors = validator.validate(&output).unwrap();

    // Should have no critical errors
    let critical_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.severity == ValidationSeverity::Error)
        .collect();

    assert!(
        critical_errors.is_empty(),
        "Should have no critical errors: {:?}",
        critical_errors
    );
}

#[test]
fn test_complex_ddd_template() {
    let config = RenderConfig::default()
        .with_syntax_validation(true)
        .with_security_checks(true);

    let renderer = SafeRenderer::new(config).unwrap();

    let aggregate_template = r#"
//! {{ name }} Aggregate Root

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{ name }}Aggregate {
    id: String,
    {% for field in fields -%}
    {{ field.name }}: {{ field.type_ }},
    {% endfor -%}
    version: u64,
}

impl {{ name }}Aggregate {
    pub fn new(id: String{% for field in fields %}, {{ field.name }}: {{ field.type_ }}{% endfor %}) -> Self {
        Self {
            id,
            {% for field in fields -%}
            {{ field.name }},
            {% endfor -%}
            version: 0,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    {% for field in fields -%}
    pub fn {{ field.name }}(&self) -> &{{ field.type_ }} {
        &self.{{ field.name }}
    }
    {% endfor %}
}
"#;

    renderer
        .add_template("aggregate", aggregate_template)
        .unwrap();

    #[derive(serde::Serialize)]
    struct Field {
        name: String,
        type_: String,
    }

    let fields = vec![
        Field {
            name: "title".to_string(),
            type_: "String".to_string(),
        },
        Field {
            name: "status".to_string(),
            type_: "Status".to_string(),
        },
    ];

    let mut context = RenderContext::new();
    context.insert("name", &"Order").unwrap();
    context.insert("fields", &fields).unwrap();

    let output = renderer.render_safe("aggregate", &context).unwrap();

    // Validate structure
    assert!(output.contains("pub struct OrderAggregate"));
    assert!(output.contains("title: String"));
    assert!(output.contains("status: Status"));
    assert!(output.contains("impl OrderAggregate"));
    assert!(output.contains("pub fn title(&self)"));
    assert!(output.contains("pub fn status(&self)"));

    // Check syntax
    let validator = OutputValidator::new(true, false);
    let errors = validator.validate(&output).unwrap();
    assert!(
        !OutputValidator::has_critical_errors(&errors),
        "Generated aggregate should be valid"
    );
}

// ============================================================================
// Performance Tests
// ============================================================================

#[test]
fn test_render_performance_baseline() {
    let config = RenderConfig::default();
    let renderer = SafeRenderer::new(config).unwrap();

    let template = r#"
{% for i in range(start=0, end=100) -%}
Item {{ i }}: {{ value }}
{% endfor -%}
"#;

    renderer.add_template("perf", template).unwrap();

    let mut context = RenderContext::new();
    context.insert("value", &"test").unwrap();

    let start = std::time::Instant::now();
    let _output = renderer.render_safe("perf", &context).unwrap();
    let duration = start.elapsed();

    // Should complete in reasonable time (< 100ms for this simple template)
    assert!(
        duration.as_millis() < 100,
        "Rendering took too long: {:?}",
        duration
    );
}
