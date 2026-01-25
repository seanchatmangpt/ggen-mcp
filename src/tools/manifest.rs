//! OpenAPI Tool Manifest Generation
//! Generates JSON schema manifest for all MCP tools for breaking change detection.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Tool manifest with version tracking and breaking change detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    /// Package version (from Cargo.toml)
    pub version: String,
    /// SHA256 hash of schemas for breaking change detection
    pub schema_hash: String,
    /// Individual tool metadata
    pub tools: Vec<ToolInfo>,
}

/// Individual tool metadata with schemas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Tool name (e.g., "list_workbooks")
    pub name: String,
    /// Tool category: core, authoring, jira, verification, vba
    pub category: String,
    /// Human-readable description
    pub description: String,
    /// JSON schema for input parameters
    pub params_schema: Value,
    /// JSON schema for response
    pub response_schema: Value,
    /// Capabilities: features provided by this tool
    pub capabilities: Vec<String>,
}

/// Manifest generator. Collects tool schemas and computes hash.
pub struct ManifestGenerator;

impl ManifestGenerator {
    /// Generate complete tool manifest from current codebase.
    pub fn generate() -> ToolManifest {
        // Core data reading tools
        let list_workbooks = Self::tool_schema(
            "list_workbooks",
            "core",
            "List spreadsheet files in workspace",
            vec!["filter", "discovery"],
        );

        let describe_workbook = Self::tool_schema(
            "describe_workbook",
            "core",
            "Describe workbook metadata and structure",
            vec!["inspection", "metadata"],
        );

        let workbook_summary = Self::tool_schema(
            "workbook_summary",
            "core",
            "Summarize workbook regions and entry points",
            vec!["analysis", "regions", "entry_points"],
        );

        let list_sheets = Self::tool_schema(
            "list_sheets",
            "core",
            "List sheets with summaries",
            vec!["inspection", "navigation"],
        );

        let sheet_overview = Self::tool_schema(
            "sheet_overview",
            "core",
            "Get narrative overview for a sheet",
            vec!["analysis", "summary"],
        );

        let read_table = Self::tool_schema(
            "read_table",
            "core",
            "Read structured data from range or table",
            vec!["reading", "data_extraction"],
        );

        let table_profile = Self::tool_schema(
            "table_profile",
            "core",
            "Analyze data distribution and patterns",
            vec!["analysis", "statistics"],
        );

        let tools = vec![
            list_workbooks,
            describe_workbook,
            workbook_summary,
            list_sheets,
            sheet_overview,
            read_table,
            table_profile,
        ];

        // Compute hash for breaking change detection
        let tools_json = serde_json::to_string(&tools).expect("Failed to serialize tools");
        let schema_hash = format!("{:x}", sha256::digest(&tools_json));

        ToolManifest {
            version: env!("CARGO_PKG_VERSION").to_string(),
            schema_hash,
            tools,
        }
    }

    /// Create tool schema with default parameter/response shapes.
    fn tool_schema(
        name: &str,
        category: &str,
        description: &str,
        capabilities: Vec<&str>,
    ) -> ToolInfo {
        ToolInfo {
            name: name.to_string(),
            category: category.to_string(),
            description: description.to_string(),
            // Simplified schema: full schemas would need reflection from actual types
            params_schema: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "title": format!("{}Params", name),
                "description": format!("Parameters for {}", name)
            }),
            response_schema: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "title": format!("{}Response", name),
                "description": format!("Response from {}", name)
            }),
            capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Compute SHA256 hash of a string.
mod sha256 {
    use sha2::{Digest, Sha256};

    pub fn digest(data: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_generation() {
        let manifest = ManifestGenerator::generate();
        assert!(!manifest.version.is_empty());
        assert!(!manifest.schema_hash.is_empty());
        assert!(!manifest.tools.is_empty());
    }

    #[test]
    fn test_manifest_consistency() {
        // Hash must be deterministic
        let m1 = ManifestGenerator::generate();
        let m2 = ManifestGenerator::generate();
        assert_eq!(m1.schema_hash, m2.schema_hash);
    }

    #[test]
    fn test_tool_categories() {
        let manifest = ManifestGenerator::generate();
        let categories: Vec<_> = manifest.tools.iter().map(|t| &t.category).collect();
        assert!(categories.iter().all(|c| c == "core"
            || c == "authoring"
            || c == "jira"
            || c == "vba"
            || c == "verification"));
    }

    #[test]
    fn test_schema_hash_length() {
        // SHA256 hex is always 64 chars
        let manifest = ManifestGenerator::generate();
        assert_eq!(manifest.schema_hash.len(), 64);
    }
}
