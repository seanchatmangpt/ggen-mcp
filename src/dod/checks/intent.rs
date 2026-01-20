use crate::dod::check::{CheckContext, DodCheck};
use crate::dod::types::*;
use anyhow::Result;
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// G8: Intent Alignment Check (Category B - WHY exists)
pub struct IntentAlignmentCheck;

#[async_trait]
impl DodCheck for IntentAlignmentCheck {
    fn id(&self) -> &str {
        "G8_INTENT"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::IntentAlignment
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Warning  // Not fatal, but important
    }

    fn description(&self) -> &str {
        "Validates that documented intent exists for changes"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();
        let mut evidence = vec![];
        let mut remediation = vec![];

        // Discover intent documents
        let intent_docs = discover_intent_documents(&context.workspace_root)?;

        if intent_docs.is_empty() {
            remediation.push("Add intent documentation: README update, ADR, or PRD".to_string());
            remediation.push("Document WHY this change is being made".to_string());

            return Ok(DodCheckResult {
                id: self.id().to_string(),
                category: self.category(),
                status: CheckStatus::Warn,
                severity: self.severity(),
                message: "No intent documentation found".to_string(),
                evidence,
                remediation,
                duration_ms: start.elapsed().as_millis() as u64,
                check_hash: String::new(),
            });
        }

        // Collect evidence
        for doc in &intent_docs {
            evidence.push(Evidence {
                kind: EvidenceKind::FileContent,
                content: format!("Intent document: {}", doc.display()),
                file_path: Some(doc.clone()),
                line_number: None,
                hash: compute_file_hash(doc)?,
            });
        }

        Ok(DodCheckResult {
            id: self.id().to_string(),
            category: self.category(),
            status: CheckStatus::Pass,
            severity: self.severity(),
            message: format!("Found {} intent document(s)", intent_docs.len()),
            evidence,
            remediation,
            duration_ms: start.elapsed().as_millis() as u64,
            check_hash: String::new(),
        })
    }
}

fn discover_intent_documents(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut docs = vec![];

    // Check for common intent documents
    let candidates = [
        "docs/PRD.md",
        "docs/ADR.md",
        "docs/DESIGN.md",
        "README.md",
        "CHANGELOG.md",
    ];

    for candidate in &candidates {
        let path = root.join(candidate);
        if path.exists() {
            // Quick check if file was recently modified (within 7 days)
            if let Ok(metadata) = fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    let now = std::time::SystemTime::now();
                    if let Ok(duration) = now.duration_since(modified) {
                        if duration.as_secs() < 7 * 24 * 60 * 60 {
                            docs.push(path);
                        }
                    }
                }
            }
        }
    }

    // Also check docs/ directory for any .md files
    let docs_dir = root.join("docs");
    if docs_dir.exists() {
        for entry in walkdir::WalkDir::new(docs_dir).max_depth(2) {
            if let Ok(entry) = entry {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("md") {
                    // Check if recently modified
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            let now = std::time::SystemTime::now();
                            if let Ok(duration) = now.duration_since(modified) {
                                if duration.as_secs() < 7 * 24 * 60 * 60 {
                                    let path = entry.path().to_path_buf();
                                    // Avoid duplicates
                                    if !docs.contains(&path) {
                                        docs.push(path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(docs)
}

fn compute_file_hash(path: &Path) -> Result<String> {
    let content = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}
