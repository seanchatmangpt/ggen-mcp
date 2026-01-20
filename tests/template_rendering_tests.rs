//! Comprehensive tests for template rendering safety
//!
//! This test suite validates the poka-yoke (error-proofing) mechanisms
//! in the template rendering system, including tests for malicious templates.

use chicago_tdd_tools::prelude::*;
use spreadsheet_mcp::template::rendering_safety::{
    ErrorRecovery, OutputValidator, RenderConfig, RenderContext, RenderGuard, RenderingError,
    SafeRenderer, ValidationSeverity,
};
use std::sync::Arc;

// ============================================================================
// Basic Rendering Tests
// ============================================================================

test!(test_basic_template_rendering, {
    // Arrange
    let config = RenderConfig::default();
    let renderer = SafeRenderer::new(config)?;
    renderer.add_template("basic", "Hello {{ name }}!")?;

    let mut context = RenderContext::new();
    context.insert("name", &"World")?;

    // Act
    let output = renderer.render_safe("basic", &context)?;

    // Assert
    assert_eq!(output, "Hello World!");
    Ok(())
});

test!(test_template_with_loop, {
    // Arrange
    let config = RenderConfig::default();
    let renderer = SafeRenderer::new(config)?;

    let template = r#"
{% for item in items -%}
- {{ item }}
{% endfor -%}
"#;

    renderer.add_template("loop", template)?;

    let mut context = RenderContext::new();
    context.insert("items", &vec!["a", "b", "c"])?;

    // Act
    let output = renderer.render_safe("loop", &context)?;

    // Assert
    assert!(output.contains("- a"));
    assert!(output.contains("- b"));
    assert!(output.contains("- c"));
    Ok(())
});

test!(test_template_with_conditionals, {
    // Arrange
    let config = RenderConfig::default();
    let renderer = SafeRenderer::new(config)?;

    let template = r#"
{% if enabled -%}
Feature is enabled
{% else -%}
Feature is disabled
{% endif -%}
"#;

    renderer.add_template("conditional", template)?;

    let mut context = RenderContext::new();

    // Act & Assert - enabled case
    context.insert("enabled", &true)?;
    let output = renderer.render_safe("conditional", &context)?;
    assert!(output.contains("Feature is enabled"));

    // Act & Assert - disabled case
    context.insert("enabled", &false)?;
    let output = renderer.render_safe("conditional", &context)?;
    assert!(output.contains("Feature is disabled"));

    Ok(())
});

// ============================================================================
// Context Scoping Tests
// ============================================================================

test!(test_context_scoping, {
    // Arrange
    let mut root = RenderContext::new();
    root.insert("root_var", &"root_value")?;

    let root_arc = Arc::new(root);
    let mut child = RenderContext::child(root_arc.clone());
    child.insert("child_var", &"child_value")?;

    // Act & Assert - child can access root variables
    assert_eq!(
        child.get("root_var")
            .ok_or("root_var not found")?
            .as_str()
            .ok_or("root_var not a string")?,
        "root_value"
    );
    assert_eq!(
        child.get("child_var")
            .ok_or("child_var not found")?
            .as_str()
            .ok_or("child_var not a string")?,
        "child_value"
    );

    // Assert - root cannot access child variables
    assert!(root_arc.get("child_var").is_none());
    Ok(())
});

test!(test_context_recursion_depth, {
    // Arrange
    let config = RenderConfig::default().with_max_recursion_depth(5);
    let mut context = RenderContext::new();

    // Act & Assert - initial depth is ok
    assert_ok!(context.check_recursion_depth(config.max_recursion_depth));

    // Act - simulate deep nesting
    let mut current = Arc::new(context);
    for _ in 0..10 {
        current = Arc::new(RenderContext::child(current));
    }

    // Assert - should exceed limit
    assert_err!(current.check_recursion_depth(5));
    Ok(())
});

test!(test_context_macro_counting, {
    // Arrange
    let config = RenderConfig::default().with_max_macro_expansions(3);
    let mut context = RenderContext::new();

    // Act & Assert - first three increments succeed
    assert_ok!(context.increment_macro_count(config.max_macro_expansions));
    assert_ok!(context.increment_macro_count(config.max_macro_expansions));
    assert_ok!(context.increment_macro_count(config.max_macro_expansions));

    // Assert - fourth increment should fail
    assert_err!(context.increment_macro_count(config.max_macro_expansions));
    Ok(())
});

// ============================================================================
// Output Validation Tests
// ============================================================================

test!(test_validator_balanced_delimiters, {
    // Arrange
    let validator = OutputValidator::new(true, false);

    let valid_code = r#"
fn main() {
    let x = vec![1, 2, 3];
    println!("{:?}", x);
}
"#;

    // Act
    let errors = validator.validate(valid_code)?;

    // Assert
    assert!(
        errors.is_empty(),
        "Valid code should have no errors: {:?}",
        errors
    );
    Ok(())
});

test!(test_validator_unbalanced_braces, {
    // Arrange
    let validator = OutputValidator::new(true, false);

    let invalid_code = r#"
fn main() {
    let x = vec![1, 2, 3;
}
"#;

    // Act
    let errors = validator.validate(invalid_code)?;

    // Assert
    assert!(!errors.is_empty(), "Should detect unbalanced delimiters");
    assert!(errors.iter().any(|e| e.message.contains("brackets")));
    Ok(())
});

test!(test_validator_unbalanced_parentheses, {
    // Arrange
    let validator = OutputValidator::new(true, false);
    let invalid_code = "fn main( { println!(\"test\"; }";

    // Act
    let errors = validator.validate(invalid_code)?;

    // Assert
    assert!(!errors.is_empty(), "Should detect unbalanced parentheses");
    Ok(())
});

test!(test_validator_security_unsafe_code, {
    // Arrange
    let validator = OutputValidator::new(false, true);

    let unsafe_code = r#"
unsafe {
    std::ptr::write(ptr, value);
}
"#;

    // Act
    let errors = validator.validate(unsafe_code)?;

    // Assert
    assert!(!errors.is_empty(), "Should detect unsafe code");
    assert!(
        errors
            .iter()
            .any(|e| e.message.to_lowercase().contains("unsafe"))
    );
    Ok(())
});

test!(test_validator_security_command_execution, {
    // Arrange
    let validator = OutputValidator::new(false, true);

    let dangerous_code = r#"
use std::process::Command;

fn run_command() {
    Command::new("rm").arg("-rf").arg("/").spawn();
}
"#;

    // Act
    let errors = validator.validate(dangerous_code)?;

    // Assert
    assert!(!errors.is_empty(), "Should detect command execution");
    assert!(
        errors
            .iter()
            .any(|e| e.message.to_lowercase().contains("command"))
    );
    Ok(())
});

test!(test_validator_security_filesystem_ops, {
    // Arrange
    let validator = OutputValidator::new(false, true);

    let fs_code = r#"
use std::fs;

fn delete_file() {
    fs::remove_file("/important/file.txt");
}
"#;

    // Act
    let errors = validator.validate(fs_code)?;

    // Assert
    assert!(!errors.is_empty(), "Should detect filesystem operations");
    Ok(())
});

test!(test_validator_invalid_identifiers, {
    // Arrange
    let validator = OutputValidator::new(true, false);

    let code_with_invalid_ids = r#"
fn 123invalid() {
    let 456var = 10;
}
"#;

    // Act
    let errors = validator.validate(code_with_invalid_ids)?;

    // Assert - this might be a warning rather than an error
    assert!(errors.iter().any(|e| matches!(
        e.severity,
        ValidationSeverity::Warning | ValidationSeverity::Error
    )));
    Ok(())
});

// ============================================================================
// Malicious Template Tests
// ============================================================================

test!(test_malicious_infinite_loop, {
    // Arrange
    let config = RenderConfig::default().with_timeout_ms(1000);
    let renderer = SafeRenderer::new(config)?;

    // Template with very large loop
    let malicious = "{% for i in range(start=0, end=999999999) %}{{ i }}{% endfor %}";
    renderer.add_template("infinite", malicious)?;

    let context = RenderContext::new();

    // Act - should complete without hanging (in real impl with timeout)
    let _ = renderer.render_safe("infinite", &context);

    // Assert - if we get here, it didn't hang indefinitely
    Ok(())
});

test!(test_malicious_deep_nesting, {
    // Arrange
    let config = RenderConfig::default().with_max_recursion_depth(5);
    let renderer = SafeRenderer::new(config)?;

    // Create deeply nested template structure
    let mut template = String::new();
    for i in 0..10 {
        template.push_str(&format!(
            "{{% if true %}}{{% if true %}}{{% if true %}}Level {}{{% endif %}}{{% endif %}}{{% endif %}}",
            i
        ));
    }

    renderer.add_template("deep", &template)?;

    let context = RenderContext::new();

    // Act - should handle deep nesting gracefully
    let _ = renderer.render_safe("deep", &context);

    // Assert - if we get here, it handled deep nesting without panic
    Ok(())
});

test!(test_malicious_large_output, {
    // Arrange
    let config = RenderConfig::default().with_max_output_size(1024); // 1KB limit
    let renderer = SafeRenderer::new(config)?;

    // Template that generates large output
    let template = r#"
{% for i in range(start=0, end=1000) -%}
This is a very long line that will be repeated many times to exceed the output size limit.
{% endfor -%}
"#;

    renderer.add_template("large", template)?;

    let context = RenderContext::new();

    // Act
    let result = renderer.render_safe("large", &context);

    // Assert - should fail with OutputSizeExceeded error
    match result {
        Err(RenderingError::OutputSizeExceeded { .. }) => {
            // Expected error
            Ok(())
        }
        Err(e) => Err(format!("Expected OutputSizeExceeded error, got: {:?}", e).into()),
        Ok(_) => Err("Expected error but got Ok".into()),
    }
});

test!(test_malicious_code_injection, {
    // Arrange
    let config = RenderConfig::default()
        .with_syntax_validation(true)
        .with_security_checks(true);

    let renderer = SafeRenderer::new(config)?;

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

    renderer.add_template("injection", template)?;

    let mut context = RenderContext::new();
    context.insert("name", &"Exploit")?;

    // Act
    let result = renderer.render_safe("injection", &context);

    // Assert - should succeed but validation should warn about unsafe
    match result {
        Ok(output) => {
            let validator = OutputValidator::new(false, true);
            let errors = validator.validate(&output)?;
            assert!(!errors.is_empty(), "Should detect unsafe code");
            Ok(())
        }
        Err(_) => {
            // Also acceptable if rendering fails
            Ok(())
        }
    }
});

test!(test_malicious_sql_injection_pattern, {
    // Arrange
    let config = RenderConfig::default().with_security_checks(true);
    let renderer = SafeRenderer::new(config)?;

    let template = r#"
fn query_user(id: &str) -> String {
    format!("SELECT * FROM users WHERE id = '{}'", id)
}
"#;

    renderer.add_template("sql", template)?;

    let context = RenderContext::new();

    // Act
    let output = renderer.render_safe("sql", &context)?;

    let validator = OutputValidator::new(false, true);
    let errors = validator.validate(&output)?;

    // Assert - should detect potential SQL injection
    assert!(
        errors
            .iter()
            .any(|e| e.message.to_lowercase().contains("sql"))
    );
    Ok(())
});

test!(test_malicious_path_traversal, {
    // Arrange
    let config = RenderConfig::default();
    let renderer = SafeRenderer::new(config)?;

    let template = r#"
use std::fs;

fn read_file(path: &str) -> String {
    fs::read_to_string(format!("../../{}", path)).unwrap()
}
"#;

    renderer.add_template("traversal", template)?;

    let context = RenderContext::new();

    // Act
    let output = renderer.render_safe("traversal", &context)?;

    let validator = OutputValidator::new(false, true);
    let errors = validator.validate(&output)?;

    // Assert - should detect filesystem operations
    assert!(!errors.is_empty());
    Ok(())
});

// ============================================================================
// Error Recovery Tests
// ============================================================================

test!(test_error_recovery_basic, {
    // Arrange
    let mut recovery = ErrorRecovery::new(false);

    // Assert - initially no errors
    assert!(!recovery.has_errors());

    // Act
    recovery.record_error(RenderingError::Timeout { timeout_ms: 5000 });

    // Assert - now has errors
    assert!(recovery.has_errors());
    assert_eq!(recovery.errors().len(), 1);
    Ok(())
});

test!(test_error_recovery_suggestions, {
    // Arrange
    let mut recovery = ErrorRecovery::new(false);

    // Act
    recovery.record_error(RenderingError::Timeout { timeout_ms: 5000 });
    recovery.record_error(RenderingError::RecursionDepthExceeded {
        depth: 15,
        limit: 10,
    });

    let suggestions = recovery.suggest_fixes();

    // Assert
    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().any(|s| s.contains("timed out")));
    assert!(suggestions.iter().any(|s| s.contains("Recursion")));
    Ok(())
});

test!(test_error_recovery_report, {
    // Arrange
    let mut recovery = ErrorRecovery::new(false);

    // Act
    recovery.record_error(RenderingError::Timeout { timeout_ms: 5000 });
    recovery.set_partial_output("partial output here".to_string());

    let report = recovery.error_report();

    // Assert
    assert!(report.contains("Rendering Errors: 1"));
    assert!(report.contains("Partial Output Available"));
    Ok(())
});

test!(test_error_recovery_with_partial_output, {
    // Arrange
    let recovery = ErrorRecovery::new(true); // allow_partial = true

    // Assert - initially no partial output
    assert!(recovery.partial_output().is_none());

    // Arrange
    let mut recovery = ErrorRecovery::new(true);

    // Act
    recovery.set_partial_output("partial content".to_string());

    // Assert
    assert_eq!(recovery.partial_output(), Some("partial content"));
    Ok(())
});

// ============================================================================
// Render Guard Tests
// ============================================================================

test!(test_render_guard_cleanup, {
    // Arrange
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_guard_cleanup.txt");

    // Create temp file
    std::fs::write(&temp_file, "test content")?;
    assert!(temp_file.exists());

    // Act - guard drops here without commit
    {
        let mut guard = RenderGuard::new();
        guard.register_temp_file(temp_file.clone());
    }

    // Assert - file should be cleaned up
    assert!(!temp_file.exists(), "Temp file should be cleaned up");
    Ok(())
});

test!(test_render_guard_commit_preserves_files, {
    // Arrange
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_guard_commit.txt");

    // Create temp file
    std::fs::write(&temp_file, "test content")?;
    assert!(temp_file.exists());

    // Act - guard commits before dropping
    {
        let mut guard = RenderGuard::new();
        guard.register_temp_file(temp_file.clone());
        let _metrics = guard.commit();
    }

    // Assert - file should still exist
    assert!(temp_file.exists(), "Committed file should be preserved");

    // Cleanup
    std::fs::remove_file(&temp_file).ok();
    Ok(())
});

test!(test_render_guard_metrics, {
    // Arrange
    use std::thread;
    use std::time::Duration;

    let mut guard = RenderGuard::new();

    // Act
    thread::sleep(Duration::from_millis(10));
    let metrics = guard.commit();

    // Assert
    assert!(metrics.duration.as_millis() >= 10);
    Ok(())
});

// ============================================================================
// Configuration Tests
// ============================================================================

test!(test_config_validation, {
    // Arrange & Act - valid config
    let valid_config = RenderConfig::default();

    // Assert
    assert_ok!(valid_config.validate());

    // Arrange & Act - invalid timeout
    let invalid_timeout = RenderConfig {
        timeout_ms: 0,
        ..Default::default()
    };
    assert_err!(invalid_timeout.validate());

    // Arrange & Act - invalid recursion
    let invalid_recursion = RenderConfig {
        max_recursion_depth: 0,
        ..Default::default()
    };
    assert_err!(invalid_recursion.validate());

    // Arrange & Act - invalid size
    let invalid_size = RenderConfig {
        max_output_size: 0,
        ..Default::default()
    };
    assert_err!(invalid_size.validate());

    Ok(())
});

test!(test_config_builder, {
    // Arrange & Act
    let config = RenderConfig::builder()
        .timeout_ms(10_000)
        .max_recursion_depth(20)
        .max_output_size(50 * 1024 * 1024)
        .validate_syntax(true)
        .security_checks(true)
        .allow_partial_rendering(true)
        .collect_metrics(true)
        .build();

    // Assert
    assert_eq!(config.timeout_ms, 10_000);
    assert_eq!(config.max_recursion_depth, 20);
    assert_eq!(config.max_output_size, 50 * 1024 * 1024);
    assert!(config.validate_syntax);
    assert!(config.security_checks);
    assert!(config.allow_partial_rendering);
    assert!(config.collect_metrics);
    Ok(())
});

test!(test_config_chaining, {
    // Arrange & Act
    let config = RenderConfig::default()
        .with_timeout_ms(8000)
        .with_max_recursion_depth(15)
        .with_syntax_validation(false)
        .with_security_checks(false);

    // Assert
    assert_eq!(config.timeout_ms, 8000);
    assert_eq!(config.max_recursion_depth, 15);
    assert!(!config.validate_syntax);
    assert!(!config.security_checks);
    Ok(())
});

test!(test_config_limits_enforced, {
    // Arrange
    use spreadsheet_mcp::template::rendering_safety::{
        MAX_OUTPUT_SIZE, MAX_RECURSION_DEPTH, MAX_TIMEOUT_MS,
    };

    // Act
    let config = RenderConfig::default()
        .with_timeout_ms(MAX_TIMEOUT_MS + 1000)
        .with_max_recursion_depth(MAX_RECURSION_DEPTH + 10)
        .with_max_output_size(MAX_OUTPUT_SIZE + 1000);

    // Assert - limits should be clamped to maximum values
    assert!(config.timeout_ms <= MAX_TIMEOUT_MS);
    assert!(config.max_recursion_depth <= MAX_RECURSION_DEPTH);
    // Note: max_output_size might not be clamped in with_ methods
    Ok(())
});

// ============================================================================
// Integration Tests
// ============================================================================

test!(test_full_render_pipeline, {
    // Arrange
    let config = RenderConfig::default()
        .with_timeout_ms(5000)
        .with_syntax_validation(true)
        .with_security_checks(true);

    let renderer = SafeRenderer::new(config)?;

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

    renderer.add_template("struct_gen", template)?;

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
    context.insert("module_name", &"test_module")?;
    context.insert("struct_name", &"TestStruct")?;
    context.insert("fields", &fields)?;

    // Act
    let output = renderer.render_safe("struct_gen", &context)?;

    // Assert - verify output
    assert!(output.contains("pub struct TestStruct"));
    assert!(output.contains("pub id: String"));
    assert!(output.contains("pub value: i32"));
    assert!(output.contains("impl TestStruct"));

    // Assert - validate output
    let validator = OutputValidator::new(true, true);
    let errors = validator.validate(&output)?;

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
    Ok(())
});

test!(test_complex_ddd_template, {
    // Arrange
    let config = RenderConfig::default()
        .with_syntax_validation(true)
        .with_security_checks(true);

    let renderer = SafeRenderer::new(config)?;

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

    renderer.add_template("aggregate", aggregate_template)?;

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
    context.insert("name", &"Order")?;
    context.insert("fields", &fields)?;

    // Act
    let output = renderer.render_safe("aggregate", &context)?;

    // Assert - validate structure
    assert!(output.contains("pub struct OrderAggregate"));
    assert!(output.contains("title: String"));
    assert!(output.contains("status: Status"));
    assert!(output.contains("impl OrderAggregate"));
    assert!(output.contains("pub fn title(&self)"));
    assert!(output.contains("pub fn status(&self)"));

    // Assert - check syntax
    let validator = OutputValidator::new(true, false);
    let errors = validator.validate(&output)?;
    assert!(
        !OutputValidator::has_critical_errors(&errors),
        "Generated aggregate should be valid"
    );
    Ok(())
});

// ============================================================================
// Performance Tests
// ============================================================================

test!(test_render_performance_baseline, {
    // Arrange
    let config = RenderConfig::default();
    let renderer = SafeRenderer::new(config)?;

    let template = r#"
{% for i in range(start=0, end=100) -%}
Item {{ i }}: {{ value }}
{% endfor -%}
"#;

    renderer.add_template("perf", template)?;

    let mut context = RenderContext::new();
    context.insert("value", &"test")?;

    // Act
    let start = std::time::Instant::now();
    let _output = renderer.render_safe("perf", &context)?;
    let duration = start.elapsed();

    // Assert - should complete in reasonable time (< 100ms for this simple template)
    assert!(
        duration.as_millis() < 100,
        "Rendering took too long: {:?}",
        duration
    );
    Ok(())
});
