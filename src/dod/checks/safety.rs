//! Category G: Safety Invariants
//!
//! G8_SECRETS: Secret detection (pattern + entropy)
//! G9_LICENSES: License header validation (optional)
//! G10_DEPENDENCIES: Dependency vulnerability scan (optional)

use crate::dod::check::{CheckContext, DodCheck};
use crate::dod::types::*;
use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;

/// G8_SECRETS: Secret detection (pattern + entropy)
pub struct SecretDetectionCheck;

#[async_trait]
impl DodCheck for SecretDetectionCheck {
    fn id(&self) -> &str {
        "G8_SECRETS"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::SafetyInvariants
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Fatal
    }

    fn description(&self) -> &str {
        "Detects hardcoded secrets using pattern matching and entropy analysis"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();
        let mut evidence = vec![];
        let mut remediation = vec![];

        // Scan critical directories
        let scan_dirs = vec!["src", "templates", "ontology", "queries"];
        let mut secrets_found = vec![];

        for dir in scan_dirs {
            let dir_path = context.workspace_root.join(dir);
            if dir_path.exists() {
                let findings = scan_directory_for_secrets(&dir_path)?;
                secrets_found.extend(findings);
            }
        }

        if secrets_found.is_empty() {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "No secrets detected".to_string(),
                evidence,
                remediation,
                duration_ms: start.elapsed().as_millis() as u64,
                check_hash: "".to_string(),
            })
        } else {
            for secret in &secrets_found {
                evidence.push(Evidence {
                    kind: EvidenceKind::FileContent,
                    content: format!(
                        "Secret detected: {} at line {}",
                        secret.pattern, secret.line
                    ),
                    file_path: Some(secret.file.clone()),
                    line_number: Some(secret.line),
                    hash: "".to_string(),
                });

                remediation.push(format!(
                    "Remove secret from {}:{} ({})",
                    secret.file.display(),
                    secret.line,
                    secret.pattern
                ));
            }

            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Fail,
                severity: self.severity(),
                message: format!("{} secret(s) detected", secrets_found.len()),
                evidence,
                remediation,
                duration_ms: start.elapsed().as_millis() as u64,
                check_hash: "".to_string(),
            })
        }
    }
}

#[derive(Debug)]
struct SecretFinding {
    file: std::path::PathBuf,
    line: usize,
    pattern: String,
}

fn scan_directory_for_secrets(dir: &Path) -> Result<Vec<SecretFinding>> {
    let mut findings = vec![];

    // Secret patterns
    let patterns = vec![
        (r"AKIA[0-9A-Z]{16}", "AWS Access Key"),
        (r"ghp_[a-zA-Z0-9]{36}", "GitHub Personal Token"),
        (r"sk_live_[0-9a-zA-Z]{24,}", "Stripe Live Key"),
        (
            r"-----BEGIN (RSA|DSA|EC|OPENSSH) PRIVATE KEY-----",
            "Private Key",
        ),
        (r"mongodb(\+srv)?://[^\s]+", "MongoDB Connection String"),
        (r"postgres://[^\s]+", "PostgreSQL Connection String"),
    ];

    let compiled: Vec<_> = patterns
        .iter()
        .map(|(pat, name)| (Regex::new(pat).unwrap(), *name))
        .collect();

    for entry in walkdir::WalkDir::new(dir).max_depth(5) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Skip binary files and large files
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.len() > 10_000_000 {
                continue; // Skip files > 10MB
            }
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue, // Skip binary files
        };

        for (line_num, line) in content.lines().enumerate() {
            for (regex, name) in &compiled {
                if regex.is_match(line) {
                    findings.push(SecretFinding {
                        file: path.to_path_buf(),
                        line: line_num + 1,
                        pattern: name.to_string(),
                    });
                }
            }

            // Entropy check: high entropy strings > 16 chars
            for word in line.split_whitespace() {
                if word.len() > 16 && calculate_entropy(word) > 4.5 {
                    // Likely a secret (high randomness)
                    findings.push(SecretFinding {
                        file: path.to_path_buf(),
                        line: line_num + 1,
                        pattern: format!("High entropy string ({}...)", &word[..8.min(word.len())]),
                    });
                }
            }
        }
    }

    Ok(findings)
}

pub fn calculate_entropy(s: &str) -> f64 {
    use std::collections::HashMap;
    let mut freq = HashMap::new();
    for c in s.chars() {
        *freq.entry(c).or_insert(0) += 1;
    }

    let len = s.len() as f64;
    let mut entropy = 0.0;
    for count in freq.values() {
        let p = *count as f64 / len;
        entropy -= p * p.log2();
    }

    entropy
}

/// G9_LICENSES: License header validation (optional)
pub struct LicenseHeaderCheck;

#[async_trait]
impl DodCheck for LicenseHeaderCheck {
    fn id(&self) -> &str {
        "G9_LICENSES"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::SafetyInvariants
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Warning // Optional check
    }

    fn description(&self) -> &str {
        "Validates license headers in generated files"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();

        // Check generated files for SPDX identifiers
        let generated_dir = context.workspace_root.join("src/generated");
        if !generated_dir.exists() {
            return Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Skip,
                severity: self.severity(),
                message: "No generated files to check".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: start.elapsed().as_millis() as u64,
                check_hash: "".to_string(),
            });
        }

        let mut missing_headers = vec![];

        for entry in walkdir::WalkDir::new(&generated_dir) {
            let entry = entry?;
            if !entry.file_type().is_file()
                || entry.path().extension().and_then(|s| s.to_str()) != Some("rs")
            {
                continue;
            }

            let content = fs::read_to_string(entry.path())?;
            let first_10_lines = content.lines().take(10).collect::<Vec<_>>().join("\n");

            if !first_10_lines.contains("SPDX-License-Identifier:") {
                missing_headers.push(entry.path().to_path_buf());
            }
        }

        if missing_headers.is_empty() {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "All generated files have license headers".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms: start.elapsed().as_millis() as u64,
                check_hash: "".to_string(),
            })
        } else {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Warn,
                severity: self.severity(),
                message: format!("{} file(s) missing license headers", missing_headers.len()),
                evidence: vec![],
                remediation: vec![
                    "Add SPDX-License-Identifier to templates".to_string(),
                    "Regenerate code with `cargo make sync`".to_string(),
                ],
                duration_ms: start.elapsed().as_millis() as u64,
                check_hash: "".to_string(),
            })
        }
    }
}

/// G10_DEPENDENCIES: Dependency vulnerability scan (optional)
pub struct DependencyRiskCheck;

#[async_trait]
impl DodCheck for DependencyRiskCheck {
    fn id(&self) -> &str {
        "G10_DEPENDENCIES"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::SafetyInvariants
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Warning // Optional, requires cargo-audit
    }

    fn description(&self) -> &str {
        "Scans dependencies for known vulnerabilities using cargo-audit"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();

        // Check if cargo-audit is installed
        let audit_check = Command::new("cargo-audit").arg("--version").output();

        if audit_check.is_err() {
            return Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Skip,
                severity: self.severity(),
                message: "cargo-audit not installed".to_string(),
                evidence: vec![],
                remediation: vec!["Install cargo-audit: cargo install cargo-audit".to_string()],
                duration_ms: start.elapsed().as_millis() as u64,
                check_hash: "".to_string(),
            });
        }

        let output = Command::new("cargo")
            .args(&["audit", "--json"])
            .current_dir(&context.workspace_root)
            .output()?;

        let duration_ms = start.elapsed().as_millis() as u64;

        if output.status.success() {
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Pass,
                severity: self.severity(),
                message: "No known vulnerabilities detected".to_string(),
                evidence: vec![],
                remediation: vec![],
                duration_ms,
                check_hash: "".to_string(),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Warn,
                severity: self.severity(),
                message: "Vulnerabilities detected in dependencies".to_string(),
                evidence: vec![Evidence {
                    kind: EvidenceKind::CommandOutput,
                    content: stderr.to_string(),
                    file_path: None,
                    line_number: None,
                    hash: "".to_string(),
                }],
                remediation: vec![
                    "Review cargo audit output".to_string(),
                    "Update vulnerable dependencies".to_string(),
                ],
                duration_ms,
                check_hash: "".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_calculation() {
        // Low entropy (repeating characters)
        let low_entropy = "aaaaaaaaaa";
        assert!(calculate_entropy(low_entropy) < 1.0);

        // High entropy (random string)
        let high_entropy = "Kd8sH3pL9xQ2mN5v";
        assert!(calculate_entropy(high_entropy) > 3.0);

        // Medium entropy
        let medium_entropy = "password123";
        let entropy = calculate_entropy(medium_entropy);
        assert!(entropy > 1.0 && entropy < 4.0);
    }

    #[test]
    fn test_secret_patterns() {
        let test_content = "AKIAIOSFODNN7EXAMPLE";
        let patterns = vec![(Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(), "AWS Access Key")];

        assert!(patterns[0].0.is_match(test_content));
    }
}
