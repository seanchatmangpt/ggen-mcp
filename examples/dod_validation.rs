//! Definition of Done Validation Examples
//!
//! Demonstrates programmatic usage of the DoD validation system.

use anyhow::{Context, Result};
use ggen_mcp::dod::check::{CheckContext, CheckRegistry, DodCheck};
use ggen_mcp::dod::executor::CheckExecutor;
use ggen_mcp::dod::profile::DodProfile;
use ggen_mcp::dod::remediation::RemediationGenerator;
use ggen_mcp::dod::types::*;
use std::path::PathBuf;

/// Example 1: Basic validation with default profile
///
/// This is the simplest way to run DoD validation programmatically.
#[tokio::main]
async fn example_basic_validation() -> Result<()> {
    println!("=== Example 1: Basic Validation ===\n");

    // Create registry with all built-in checks
    let registry = ggen_mcp::dod::checks::create_registry();

    // Use default development profile
    let profile = DodProfile::default_dev();

    // Create executor
    let executor = CheckExecutor::new(registry, profile);

    // Create context pointing to workspace root
    let context = CheckContext::new(PathBuf::from("."));

    // Execute all checks
    println!("Running DoD validation...");
    let results = executor
        .execute_all(&context)
        .await
        .context("Failed to execute checks")?;

    // Print summary
    let passed = results
        .iter()
        .filter(|r| r.status == CheckStatus::Pass)
        .count();
    let failed = results
        .iter()
        .filter(|r| r.status == CheckStatus::Fail)
        .count();
    let warned = results
        .iter()
        .filter(|r| r.status == CheckStatus::Warn)
        .count();

    println!("\nResults:");
    println!("  Passed: {}", passed);
    println!("  Failed: {}", failed);
    println!("  Warned: {}", warned);
    println!("  Total:  {}", results.len());

    // Print individual results
    println!("\nCheck Details:");
    for result in &results {
        let icon = match result.status {
            CheckStatus::Pass => "✓",
            CheckStatus::Fail => "✗",
            CheckStatus::Warn => "⚠",
            CheckStatus::Skip => "⊘",
        };
        println!(
            "  {} {} ({:?}) - {}ms",
            icon, result.id, result.status, result.duration_ms
        );
    }

    Ok(())
}

/// Example 2: Custom profile with strict thresholds
///
/// Demonstrates creating and using a custom profile.
#[tokio::main]
async fn example_custom_profile() -> Result<()> {
    println!("=== Example 2: Custom Profile ===\n");

    let registry = ggen_mcp::dod::checks::create_registry();

    // Create custom profile
    let mut profile = DodProfile::default_dev();
    profile.name = "custom-strict".to_string();
    profile.description = "Custom strict profile for CI".to_string();

    // Customize thresholds
    profile.thresholds.min_readiness_score = 85.0; // Require 85% score
    profile.thresholds.max_warnings = 5; // Max 5 warnings
    profile.thresholds.require_all_tests_pass = true; // No test failures
    profile.thresholds.fail_on_clippy_warnings = true; // Clippy warnings fail

    // Customize category weights
    profile.category_weights.clear();
    profile
        .category_weights
        .insert("BuildCorrectness".to_string(), 0.30);
    profile
        .category_weights
        .insert("TestTruth".to_string(), 0.30);
    profile
        .category_weights
        .insert("GgenPipeline".to_string(), 0.25);
    profile
        .category_weights
        .insert("SafetyInvariants".to_string(), 0.15);

    // Validate profile
    profile
        .validate()
        .context("Invalid profile configuration")?;

    println!("Custom Profile:");
    println!("  Name: {}", profile.name);
    println!("  Min Score: {}", profile.thresholds.min_readiness_score);
    println!("  Max Warnings: {}", profile.thresholds.max_warnings);
    println!(
        "  Require All Tests Pass: {}",
        profile.thresholds.require_all_tests_pass
    );

    let executor = CheckExecutor::new(registry, profile);
    let context = CheckContext::new(PathBuf::from("."));

    let results = executor.execute_all(&context).await?;

    println!("\nExecuted {} checks", results.len());

    Ok(())
}

/// Example 3: Execute single check
///
/// Run a specific check by ID instead of all checks.
#[tokio::main]
async fn example_single_check() -> Result<()> {
    println!("=== Example 3: Single Check ===\n");

    let registry = ggen_mcp::dod::checks::create_registry();
    let profile = DodProfile::default_dev();
    let executor = CheckExecutor::new(registry, profile);
    let context = CheckContext::new(PathBuf::from("."));

    // Execute only BUILD_FMT check
    println!("Running BUILD_FMT check...");
    let result = executor
        .execute_one("BUILD_FMT", &context)
        .await
        .context("Failed to execute BUILD_FMT")?;

    println!("\nResult:");
    println!("  ID: {}", result.id);
    println!("  Status: {:?}", result.status);
    println!("  Message: {}", result.message);
    println!("  Duration: {}ms", result.duration_ms);

    if !result.remediation.is_empty() {
        println!("\nRemediation:");
        for (i, step) in result.remediation.iter().enumerate() {
            println!("  {}. {}", i + 1, step);
        }
    }

    Ok(())
}

/// Example 4: Custom check implementation
///
/// Demonstrates implementing a custom DoD check.
#[tokio::main]
async fn example_custom_check() -> Result<()> {
    println!("=== Example 4: Custom Check ===\n");

    use async_trait::async_trait;

    /// Custom check: Validates README.md exists and is not empty
    struct ReadmeCheck;

    #[async_trait]
    impl DodCheck for ReadmeCheck {
        fn id(&self) -> &str {
            "CUSTOM_README"
        }

        fn category(&self) -> CheckCategory {
            CheckCategory::WorkspaceIntegrity
        }

        fn severity(&self) -> CheckSeverity {
            CheckSeverity::Warning
        }

        fn description(&self) -> &str {
            "Validates README.md exists and is not empty"
        }

        async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
            let start = std::time::Instant::now();

            let readme_path = context.workspace_root.join("README.md");

            let (status, message, remediation) = if readme_path.exists() {
                let content =
                    std::fs::read_to_string(&readme_path).context("Failed to read README.md")?;

                if content.trim().is_empty() {
                    (
                        CheckStatus::Fail,
                        "README.md exists but is empty".to_string(),
                        vec!["Add content to README.md".to_string()],
                    )
                } else if content.len() < 100 {
                    (
                        CheckStatus::Warn,
                        format!("README.md is very short ({} bytes)", content.len()),
                        vec!["Expand README.md with more information".to_string()],
                    )
                } else {
                    (
                        CheckStatus::Pass,
                        format!("README.md exists ({} bytes)", content.len()),
                        vec![],
                    )
                }
            } else {
                (
                    CheckStatus::Fail,
                    "README.md not found".to_string(),
                    vec!["Create README.md with project documentation".to_string()],
                )
            };

            let duration_ms = start.elapsed().as_millis() as u64;

            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status,
                severity: self.severity(),
                message,
                evidence: vec![],
                remediation,
                duration_ms,
                check_hash: String::new(),
            })
        }
    }

    // Create registry and add custom check
    let mut registry = CheckRegistry::new();
    registry.register(Box::new(ReadmeCheck));

    // Create profile that includes our custom check
    let mut profile = DodProfile::default_dev();
    profile.required_checks.clear();
    profile.required_checks.push("CUSTOM_README".to_string());

    let executor = CheckExecutor::new(registry, profile);
    let context = CheckContext::new(PathBuf::from("."));

    println!("Running custom README check...");
    let results = executor.execute_all(&context).await?;

    for result in &results {
        println!("\nResult:");
        println!("  ID: {}", result.id);
        println!("  Status: {:?}", result.status);
        println!("  Message: {}", result.message);

        if !result.remediation.is_empty() {
            println!("\nRemediation:");
            for step in &result.remediation {
                println!("  - {}", step);
            }
        }
    }

    Ok(())
}

/// Example 5: Remediation suggestions
///
/// Generate actionable fix suggestions from check results.
#[tokio::main]
async fn example_remediation() -> Result<()> {
    println!("=== Example 5: Remediation Suggestions ===\n");

    let registry = ggen_mcp::dod::checks::create_registry();
    let profile = DodProfile::default_dev();
    let executor = CheckExecutor::new(registry, profile);
    let context = CheckContext::new(PathBuf::from("."));

    // Execute checks
    let results = executor.execute_all(&context).await?;

    // Generate remediation suggestions
    let suggestions = RemediationGenerator::generate(&results);

    if suggestions.is_empty() {
        println!("✓ All checks passed! No remediation needed.");
    } else {
        println!(
            "Found {} issues requiring remediation:\n",
            suggestions.len()
        );

        for (i, suggestion) in suggestions.iter().enumerate() {
            println!(
                "{}. {} (Priority: {:?})",
                i + 1,
                suggestion.title,
                suggestion.priority
            );
            println!("   Check: {}", suggestion.check_id);

            if !suggestion.steps.is_empty() {
                println!("   Steps:");
                for step in &suggestion.steps {
                    println!("     - {}", step);
                }
            }

            if let Some(cmd) = &suggestion.automation {
                println!("   Automation: {}", cmd);
            }

            println!();
        }
    }

    Ok(())
}

/// Example 6: Category filtering
///
/// Execute only checks from specific categories.
#[tokio::main]
async fn example_category_filtering() -> Result<()> {
    println!("=== Example 6: Category Filtering ===\n");

    let registry = ggen_mcp::dod::checks::create_registry();

    // Get only build checks
    let build_checks = registry.get_by_category(CheckCategory::BuildCorrectness);

    println!("Build Correctness Checks:");
    for check in build_checks {
        println!("  - {} ({})", check.id(), check.description());
    }

    // Get only test checks
    let test_checks = registry.get_by_category(CheckCategory::TestTruth);

    println!("\nTest Truth Checks:");
    for check in test_checks {
        println!("  - {} ({})", check.id(), check.description());
    }

    // Create profile with only build checks
    let mut profile = DodProfile::default_dev();
    profile.required_checks.clear();
    profile.required_checks.push("BUILD_FMT".to_string());
    profile.required_checks.push("BUILD_CLIPPY".to_string());
    profile.required_checks.push("BUILD_CHECK".to_string());

    let executor = CheckExecutor::new(registry, profile);
    let context = CheckContext::new(PathBuf::from("."));

    println!("\nRunning build checks only...");
    let results = executor.execute_all(&context).await?;

    println!("Executed {} checks", results.len());

    Ok(())
}

/// Example 7: Evidence collection
///
/// Access evidence from check results.
#[tokio::main]
async fn example_evidence() -> Result<()> {
    println!("=== Example 7: Evidence Collection ===\n");

    let registry = ggen_mcp::dod::checks::create_registry();
    let profile = DodProfile::default_dev();
    let executor = CheckExecutor::new(registry, profile);
    let context = CheckContext::new(PathBuf::from("."));

    let results = executor.execute_all(&context).await?;

    // Find checks with evidence
    for result in &results {
        if !result.evidence.is_empty() {
            println!("Check: {} ({})", result.id, result.status_debug());
            println!("Evidence:");

            for (i, evidence) in result.evidence.iter().enumerate() {
                println!("  {}. Kind: {:?}", i + 1, evidence.kind);

                if let Some(path) = &evidence.file_path {
                    println!("     File: {:?}", path);
                }

                if let Some(line) = evidence.line_number {
                    println!("     Line: {}", line);
                }

                // Truncate content for display
                let content_preview = if evidence.content.len() > 100 {
                    format!(
                        "{}... ({} bytes)",
                        &evidence.content[..100],
                        evidence.content.len()
                    )
                } else {
                    evidence.content.clone()
                };

                println!("     Content: {}", content_preview);

                if !evidence.hash.is_empty() {
                    println!("     Hash: {}", evidence.hash);
                }

                println!();
            }
        }
    }

    Ok(())
}

/// Example 8: Profile from file
///
/// Load a profile from a TOML file.
#[tokio::main]
async fn example_profile_from_file() -> Result<()> {
    println!("=== Example 8: Profile from File ===\n");

    // Create example profile file
    let profile_toml = r#"
name = "example-profile"
description = "Example profile loaded from TOML"

required_checks = [
    "BUILD_FMT",
    "BUILD_CHECK",
    "TEST_UNIT",
]

optional_checks = [
    "BUILD_CLIPPY",
]

[category_weights]
BuildCorrectness = 0.50
TestTruth = 0.50

[parallelism]
mode = "auto"

[timeouts_ms]
build = 600000
tests = 900000
ggen = 300000
default = 60000

[thresholds]
min_readiness_score = 75.0
max_warnings = 15
require_all_tests_pass = false
fail_on_clippy_warnings = false
"#;

    // Write temporary profile file
    std::fs::write("example-profile.toml", profile_toml).context("Failed to write profile file")?;

    // Load profile from file
    let profile =
        DodProfile::load_from_file("example-profile.toml").context("Failed to load profile")?;

    println!("Loaded Profile:");
    println!("  Name: {}", profile.name);
    println!("  Description: {}", profile.description);
    println!("  Required Checks: {}", profile.required_checks.len());
    println!("  Optional Checks: {}", profile.optional_checks.len());
    println!("  Min Score: {}", profile.thresholds.min_readiness_score);

    // Clean up
    std::fs::remove_file("example-profile.toml")?;

    Ok(())
}

/// Example 9: Timeout handling
///
/// Configure and handle check timeouts.
#[tokio::main]
async fn example_timeout_handling() -> Result<()> {
    println!("=== Example 9: Timeout Handling ===\n");

    let registry = ggen_mcp::dod::checks::create_registry();

    // Create profile with short timeouts
    let mut profile = DodProfile::default_dev();
    profile.timeouts_ms.build = 100; // 100ms - very short!
    profile.timeouts_ms.tests = 100;
    profile.timeouts_ms.ggen = 100;
    profile.timeouts_ms.default = 100;

    let executor = CheckExecutor::new(registry, profile);
    let context = CheckContext::new(PathBuf::from("."));

    println!("Running checks with very short timeouts...");
    let results = executor.execute_all(&context).await?;

    // Find checks that timed out
    let timed_out: Vec<_> = results
        .iter()
        .filter(|r| r.message.contains("timed out"))
        .collect();

    if timed_out.is_empty() {
        println!("✓ All checks completed within timeout");
    } else {
        println!("⚠ {} checks timed out:", timed_out.len());
        for result in timed_out {
            println!("  - {}: {}", result.id, result.message);
        }
    }

    Ok(())
}

/// Example 10: Serial vs parallel execution
///
/// Compare serial and parallel execution modes.
#[tokio::main]
async fn example_parallelism() -> Result<()> {
    use ggen_mcp::dod::profile::ParallelismConfig;
    use std::time::Instant;

    println!("=== Example 10: Parallelism ===\n");

    let registry = ggen_mcp::dod::checks::create_registry();
    let context = CheckContext::new(PathBuf::from("."));

    // Serial execution
    let mut profile_serial = DodProfile::default_dev();
    profile_serial.parallelism = ParallelismConfig::Serial;

    let executor_serial = CheckExecutor::new(registry.clone(), profile_serial);

    println!("Running checks in SERIAL mode...");
    let start = Instant::now();
    let results_serial = executor_serial.execute_all(&context).await?;
    let duration_serial = start.elapsed();

    println!(
        "Serial: {} checks in {:.2}s",
        results_serial.len(),
        duration_serial.as_secs_f64()
    );

    // Parallel execution
    let mut profile_parallel = DodProfile::default_dev();
    profile_parallel.parallelism = ParallelismConfig::Auto;

    let executor_parallel = CheckExecutor::new(registry, profile_parallel);

    println!("\nRunning checks in PARALLEL mode...");
    let start = Instant::now();
    let results_parallel = executor_parallel.execute_all(&context).await?;
    let duration_parallel = start.elapsed();

    println!(
        "Parallel: {} checks in {:.2}s",
        results_parallel.len(),
        duration_parallel.as_secs_f64()
    );

    if duration_serial > duration_parallel {
        let speedup = duration_serial.as_secs_f64() / duration_parallel.as_secs_f64();
        println!("\n✓ Parallel execution was {:.2}x faster", speedup);
    }

    Ok(())
}

// Helper trait for pretty printing
trait StatusDebug {
    fn status_debug(&self) -> String;
}

impl StatusDebug for DodCheckResult {
    fn status_debug(&self) -> String {
        match self.status {
            CheckStatus::Pass => "PASS".to_string(),
            CheckStatus::Fail => "FAIL".to_string(),
            CheckStatus::Warn => "WARN".to_string(),
            CheckStatus::Skip => "SKIP".to_string(),
        }
    }
}

/// Main function - run all examples
#[tokio::main]
async fn main() -> Result<()> {
    println!("DoD Validation Examples\n");
    println!("Run individual examples with:");
    println!("  cargo run --example dod_validation --features example-1");
    println!();

    // Uncomment to run a specific example:
    // example_basic_validation().await?;
    // example_custom_profile().await?;
    // example_single_check().await?;
    // example_custom_check().await?;
    // example_remediation().await?;
    // example_category_filtering().await?;
    // example_evidence().await?;
    // example_profile_from_file().await?;
    // example_timeout_handling().await?;
    // example_parallelism().await?;

    Ok(())
}
