use crate::dod::check::{CheckContext, DodCheck};
use crate::dod::types::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// G0: Workspace Integrity Check (Category A)
pub struct WorkspaceIntegrityCheck;

#[async_trait]
impl DodCheck for WorkspaceIntegrityCheck {
    fn id(&self) -> &str {
        "G0_WORKSPACE"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::WorkspaceIntegrity
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Fatal
    }

    fn description(&self) -> &str {
        "Validates workspace structure, paths, and environment"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();
        let mut evidence = vec![];
        let mut messages = vec![];
        let mut remediation = vec![];

        // Check 1: Repo root exists
        if !context.workspace_root.exists() {
            return Ok(self.fail_result(
                "Workspace root does not exist",
                vec!["Ensure the path points to a valid directory".to_string()],
                start,
            ));
        }
        messages.push("Workspace root exists".to_string());

        // Check 2: Required directories
        let required_dirs = ["src", "tests", "ontology", "templates", "queries"];
        for dir in &required_dirs {
            let dir_path = context.workspace_root.join(dir);
            if !dir_path.exists() {
                remediation.push(format!("Create missing directory: {}", dir));
                messages.push(format!("Missing required directory: {}", dir));
            } else {
                evidence.push(Evidence {
                    kind: EvidenceKind::FileContent,
                    content: format!("Directory exists: {}", dir),
                    file_path: Some(dir_path),
                    line_number: None,
                    hash: String::new(),
                });
            }
        }

        // Check 3: Cargo.toml exists (Rust project marker)
        let cargo_toml = context.workspace_root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Ok(self.fail_result(
                "Cargo.toml not found - not a valid Rust project",
                vec!["Initialize Cargo project with `cargo init`".to_string()],
                start,
            ));
        }

        // Check 4: Cargo.lock exists (dependencies locked)
        let cargo_lock = context.workspace_root.join("Cargo.lock");
        if !cargo_lock.exists() {
            remediation.push("Run `cargo build` to generate Cargo.lock".to_string());
            messages.push("Warning: Cargo.lock missing".to_string());
        } else {
            let lock_hash = compute_file_hash(&cargo_lock)?;
            evidence.push(Evidence {
                kind: EvidenceKind::Hash,
                content: format!("Cargo.lock hash: {}", lock_hash),
                file_path: Some(cargo_lock),
                line_number: None,
                hash: lock_hash,
            });
        }

        // Check 5: No path traversal in symlinks
        if let Err(e) = check_symlink_safety(&context.workspace_root) {
            return Ok(self.fail_result(
                &format!("Unsafe symlink detected: {}", e),
                vec!["Remove or fix symlinks that escape workspace".to_string()],
                start,
            ));
        }

        // Check 6: Environment capture
        let env_fingerprint = capture_environment()?;
        evidence.push(Evidence {
            kind: EvidenceKind::Metric,
            content: env_fingerprint,
            file_path: None,
            line_number: None,
            hash: String::new(),
        });

        let duration_ms = start.elapsed().as_millis() as u64;
        let status = if remediation.is_empty() {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        };

        Ok(DodCheckResult {
            id: self.id().to_string(),
            category: self.category(),
            status,
            severity: self.severity(),
            message: messages.join("; "),
            evidence,
            remediation,
            duration_ms,
            check_hash: compute_check_hash(&evidence),
        })
    }
}

impl WorkspaceIntegrityCheck {
    fn fail_result(
        &self,
        message: &str,
        remediation: Vec<String>,
        start: std::time::Instant,
    ) -> DodCheckResult {
        DodCheckResult {
            id: self.id().to_string(),
            category: self.category(),
            status: CheckStatus::Fail,
            severity: self.severity(),
            message: message.to_string(),
            evidence: vec![],
            remediation,
            duration_ms: start.elapsed().as_millis() as u64,
            check_hash: String::new(),
        }
    }
}

fn compute_file_hash(path: &Path) -> Result<String> {
    let content = fs::read(path).context("Failed to read file")?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

fn check_symlink_safety(root: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(root).follow_links(false).max_depth(3) {
        let entry = entry?;
        if entry.path_is_symlink() {
            let target = fs::read_link(entry.path())?;
            // Check if target is absolute and outside workspace
            if target.is_absolute() {
                let canonical = target.canonicalize()?;
                let root_canonical = root.canonicalize()?;
                if !canonical.starts_with(&root_canonical) {
                    anyhow::bail!("Symlink escapes workspace: {:?}", entry.path());
                }
            } else {
                // Relative symlink - resolve and check
                let resolved = entry.path().parent().unwrap_or(root).join(&target);
                if let Ok(canonical) = resolved.canonicalize() {
                    let root_canonical = root.canonicalize()?;
                    if !canonical.starts_with(&root_canonical) {
                        anyhow::bail!("Symlink escapes workspace: {:?}", entry.path());
                    }
                }
            }
        }
    }
    Ok(())
}

fn capture_environment() -> Result<String> {
    let rustc_version = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string());

    Ok(format!(
        "rustc: {}, platform: {}",
        rustc_version.trim(),
        std::env::consts::OS
    ))
}

fn compute_check_hash(evidence: &[Evidence]) -> String {
    let mut hasher = Sha256::new();
    for ev in evidence {
        hasher.update(ev.hash.as_bytes());
        hasher.update(ev.content.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}
