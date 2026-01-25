//! Category C: Tool Registry Consistency Check
//!
//! Validates that tools are consistently registered across:
//! - src/server.rs (tool registration)
//! - src/tools/ (tool implementation)
//! - CLAUDE.md (documentation)

use crate::dod::check::{CheckContext, DodCheck};
use crate::dod::types::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// WHAT_TOOL_REGISTRY: Tool registration consistency check
pub struct ToolRegistryCheck;

#[async_trait]
impl DodCheck for ToolRegistryCheck {
    fn id(&self) -> &str {
        "WHAT_TOOL_REGISTRY"
    }

    fn category(&self) -> CheckCategory {
        CheckCategory::ToolRegistry
    }

    fn severity(&self) -> CheckSeverity {
        CheckSeverity::Fatal
    }

    fn description(&self) -> &str {
        "Validates tool registration consistency: server.rs ↔ tools/ ↔ CLAUDE.md"
    }

    async fn execute(&self, context: &CheckContext) -> Result<DodCheckResult> {
        let start = std::time::Instant::now();
        let mut evidence = vec![];
        let mut remediation = vec![];

        // Extract tool names from different sources
        let registered_tools = extract_registered_tools(&context.workspace_root)?;
        let implemented_tools = extract_implemented_tools(&context.workspace_root)?;
        let documented_tools = extract_documented_tools(&context.workspace_root)?;

        // Find mismatches
        let only_registered: Vec<_> = registered_tools
            .difference(&implemented_tools)
            .cloned()
            .collect();
        let only_implemented: Vec<_> = implemented_tools
            .difference(&registered_tools)
            .cloned()
            .collect();
        let missing_docs: Vec<_> = registered_tools
            .difference(&documented_tools)
            .cloned()
            .collect();

        let mut messages = vec![];

        if !only_registered.is_empty() {
            messages.push(format!(
                "Tools registered but not implemented: {:?}",
                only_registered
            ));
            remediation.push("Implement missing tool handlers in src/tools/".to_string());
        }

        if !only_implemented.is_empty() {
            messages.push(format!(
                "Tools implemented but not registered: {:?}",
                only_implemented
            ));
            remediation.push("Register tools in src/server.rs".to_string());
        }

        if !missing_docs.is_empty() {
            messages.push(format!(
                "Tools missing from documentation: {:?}",
                missing_docs
            ));
            remediation.push("Add tool descriptions to CLAUDE.md".to_string());
        }

        // Collect evidence
        evidence.push(Evidence {
            kind: EvidenceKind::Metric,
            content: format!(
                "Registered: {}, Implemented: {}, Documented: {}",
                registered_tools.len(),
                implemented_tools.len(),
                documented_tools.len()
            ),
            file_path: None,
            line_number: None,
            hash: String::new(),
        });

        let status = if remediation.is_empty() {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        };

        Ok(DodCheckResult {
            id: self.id().to_string(),
            category: self.category(),
            status,
            severity: self.severity(),
            message: if messages.is_empty() {
                "Tool registry consistent".to_string()
            } else {
                messages.join("; ")
            },
            evidence,
            remediation,
            duration_ms: start.elapsed().as_millis() as u64,
            check_hash: String::new(),
        })
    }
}

fn extract_registered_tools(root: &Path) -> Result<HashSet<String>> {
    let server_rs = root.join("src/server.rs");
    let content = fs::read_to_string(&server_rs).context("Failed to read src/server.rs")?;

    let mut tools = HashSet::new();

    // Parse tool registrations: .route("tool_name", handler)
    for line in content.lines() {
        if line.contains(".route(") {
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    let tool_name = &line[start + 1..start + 1 + end];
                    if !tool_name.is_empty() {
                        tools.insert(tool_name.to_string());
                    }
                }
            }
        }
    }

    Ok(tools)
}

fn extract_implemented_tools(root: &Path) -> Result<HashSet<String>> {
    let tools_dir = root.join("src/tools");
    if !tools_dir.exists() {
        return Ok(HashSet::new());
    }

    let mut tools = HashSet::new();

    // Walk tools directory and find handler functions
    for entry in walkdir::WalkDir::new(&tools_dir).max_depth(2) {
        let entry = entry?;
        if entry.path().extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = fs::read_to_string(entry.path())?;

            // Look for public async fn that return Result
            for line in content.lines() {
                if (line.contains("pub async fn") || line.contains("pub fn"))
                    && !line.contains("//")
                {
                    if let Some(fn_start) = line.find("fn ") {
                        if let Some(paren) = line[fn_start..].find('(') {
                            let fn_name = line[fn_start + 3..fn_start + paren].trim();
                            if !fn_name.is_empty() {
                                tools.insert(fn_name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(tools)
}

fn extract_documented_tools(root: &Path) -> Result<HashSet<String>> {
    let claude_md = root.join("CLAUDE.md");
    if !claude_md.exists() {
        return Ok(HashSet::new());
    }

    let content = fs::read_to_string(&claude_md)?;
    let mut tools = HashSet::new();

    // Parse tool names from CLAUDE.md - look for code blocks with tool names
    let mut in_tools_section = false;
    for line in content.lines() {
        if line.contains("MCP Tools") || line.contains("### Spreadsheet Operations") {
            in_tools_section = true;
        }

        if in_tools_section {
            // Look for tool names in backticks or code blocks
            if line.contains('`') {
                for part in line.split('`') {
                    if part.contains('_') && !part.contains(' ') {
                        tools.insert(part.to_string());
                    }
                }
            }
        }
    }

    Ok(tools)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_tool_registry_check() {
        let check = ToolRegistryCheck;
        let context = CheckContext::new(PathBuf::from("/home/user/ggen-mcp"), ValidationMode::Fast);

        let result = check.execute(&context).await.unwrap();
        assert_eq!(result.category, CheckCategory::ToolRegistry);
    }
}
