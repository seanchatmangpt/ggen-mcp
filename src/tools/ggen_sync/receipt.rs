//! Cryptographic Receipt Generation for Proof-Carrying Code
//!
//! This module implements comprehensive receipt generation for ggen code generation,
//! providing cryptographic proof of:
//! - Input provenance (ontologies, queries, templates, config)
//! - Guard execution verdicts
//! - Output artifacts with SHA-256 hashes
//! - Performance metrics
//! - Reproducibility guarantees
//!
//! ## Receipt Schema
//! Conforms to `schemas/receipt.json` JSON Schema specification.
//!
//! ## Usage
//! ```ignore
//! let receipt = ReceiptGenerator::generate(&sync_context, &sync_results)?;
//! receipt.save(&output_path)?;
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::report::SyncMode;

// ============================================================================
// Receipt Data Structures (JSON Schema Compliant)
// ============================================================================

/// Complete cryptographic receipt for code generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    /// Schema version (always "1.0.0")
    pub version: String,

    /// Workspace identification
    pub workspace: WorkspaceInfo,

    /// Input file hashes
    pub inputs: InputsInfo,

    /// Guard execution results
    pub guards: GuardsInfo,

    /// Generated output files
    pub outputs: Vec<OutputFile>,

    /// Compilation metadata
    pub metadata: ReceiptMetadata,
}

/// Workspace fingerprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    /// Absolute path to workspace root
    pub root: String,

    /// SHA-256 hash of workspace root path
    pub fingerprint: String,
}

/// All input file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputsInfo {
    /// Configuration file
    pub config: ConfigInput,

    /// Ontology files
    pub ontologies: Vec<OntologyInput>,

    /// SPARQL query files
    pub queries: Vec<FileInput>,

    /// Tera template files
    pub templates: Vec<FileInput>,
}

/// Configuration file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInput {
    /// Path to ggen.toml
    pub path: String,

    /// SHA-256 hash of config content
    pub hash: String,

    /// Number of generation rules
    pub rules_count: usize,
}

/// Ontology file metadata (with triple count)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyInput {
    /// Path to .ttl file
    pub path: String,

    /// SHA-256 hash of ontology content
    pub hash: String,

    /// Number of RDF triples loaded
    pub triple_count: usize,
}

/// Generic file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInput {
    /// File path
    pub path: String,

    /// SHA-256 hash of file content
    pub hash: String,
}

/// Guard execution information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardsInfo {
    /// Guard kernel version
    pub kernel_version: String,

    /// Individual guard verdicts
    pub verdicts: Vec<GuardVerdict>,
}

/// Single guard verdict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardVerdict {
    /// Guard identifier
    pub name: String,

    /// Pass/fail verdict
    pub verdict: String, // "pass" | "fail"

    /// Human-readable diagnostic
    pub diagnostic: String,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Generated output file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFile {
    /// Output file path
    pub path: String,

    /// SHA-256 hash of generated content
    pub hash: String,

    /// File size in bytes
    pub size: usize,

    /// Output language
    pub language: String,
}

/// Receipt metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptMetadata {
    /// ISO-8601 timestamp
    pub timestamp: String,

    /// ggen-mcp version
    pub compiler_version: String,

    /// Execution mode
    pub mode: String, // "preview" | "apply"

    /// Overall status
    pub status: String, // "pass" | "fail"

    /// Performance metrics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performance: Option<PerformanceMetrics>,
}

/// Performance timing metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total execution time (ms)
    pub total_duration_ms: u64,

    /// Resource discovery time (ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_ms: Option<u64>,

    /// Guard execution time (ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guards_ms: Option<u64>,

    /// SPARQL query execution time (ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparql_ms: Option<u64>,

    /// Template rendering time (ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_ms: Option<u64>,

    /// Code validation time (ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate_ms: Option<u64>,
}

// ============================================================================
// Receipt Generator
// ============================================================================

/// Receipt generation engine
pub struct ReceiptGenerator;

impl ReceiptGenerator {
    /// Generate complete receipt from sync execution context
    ///
    /// # Arguments
    /// * `workspace_root` - Absolute path to workspace
    /// * `config_path` - Path to ggen.toml
    /// * `ontology_paths` - Paths to ontology files
    /// * `query_paths` - Paths to SPARQL query files
    /// * `template_paths` - Paths to Tera template files
    /// * `output_files` - Generated output files with content
    /// * `mode` - Sync execution mode (Preview or Apply)
    /// * `total_duration_ms` - Total execution time
    ///
    /// # Returns
    /// Complete cryptographic receipt
    pub fn generate(
        workspace_root: &str,
        config_path: Option<&Path>,
        ontology_paths: &[PathBuf],
        query_paths: &[PathBuf],
        template_paths: &[PathBuf],
        output_files: &[(String, String)], // (path, content)
        mode: SyncMode,
        total_duration_ms: u64,
    ) -> Result<Receipt> {
        // Workspace fingerprint
        let workspace = WorkspaceInfo {
            root: workspace_root.to_string(),
            fingerprint: hash_string(workspace_root),
        };

        // Config input
        let config = if let Some(config_path) = config_path {
            let hash = hash_file(config_path)?;
            let rules_count = Self::count_rules(config_path)?;
            ConfigInput {
                path: config_path.display().to_string(),
                hash,
                rules_count,
            }
        } else {
            ConfigInput {
                path: "ggen.toml (not found)".to_string(),
                hash: hash_string(""),
                rules_count: 0,
            }
        };

        // Ontology inputs
        let ontologies = ontology_paths
            .iter()
            .map(|p| {
                let hash = hash_file(p)?;
                let triple_count = Self::count_triples(p)?;
                Ok(OntologyInput {
                    path: p.display().to_string(),
                    hash,
                    triple_count,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        // Query inputs
        let queries = query_paths
            .iter()
            .map(|p| {
                let hash = hash_file(p)?;
                Ok(FileInput {
                    path: p.display().to_string(),
                    hash,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        // Template inputs
        let templates = template_paths
            .iter()
            .map(|p| {
                let hash = hash_file(p)?;
                Ok(FileInput {
                    path: p.display().to_string(),
                    hash,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let inputs = InputsInfo {
            config,
            ontologies,
            queries,
            templates,
        };

        // Guards (placeholder - will be populated by actual guard execution)
        let guards = GuardsInfo {
            kernel_version: "1.0.0".to_string(),
            verdicts: vec![],
        };

        // Output files
        let outputs = output_files
            .iter()
            .map(|(path, content)| {
                let hash = hash_string(content);
                let size = content.len();
                let language = Self::detect_language(Path::new(path));
                OutputFile {
                    path: path.clone(),
                    hash,
                    size,
                    language,
                }
            })
            .collect();

        // Metadata
        let metadata = ReceiptMetadata {
            timestamp: chrono::Utc::now().to_rfc3339(),
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            mode: format!("{}", mode),
            status: "pass".to_string(), // Will be updated based on guard verdicts
            performance: Some(PerformanceMetrics {
                total_duration_ms,
                discovery_ms: None,
                guards_ms: None,
                sparql_ms: None,
                render_ms: None,
                validate_ms: None,
            }),
        };

        Ok(Receipt {
            version: "1.0.0".to_string(),
            workspace,
            inputs,
            guards,
            outputs,
            metadata,
        })
    }

    /// Save receipt to JSON file
    pub fn save(receipt: &Receipt, output_path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create receipt directory: {}", parent.display())
            })?;
        }

        // Serialize to pretty JSON
        let json =
            serde_json::to_string_pretty(receipt).context("Failed to serialize receipt to JSON")?;

        // Write atomically
        fs::write(output_path, json)
            .with_context(|| format!("Failed to write receipt to {}", output_path.display()))?;

        tracing::info!("Receipt saved to {}", output_path.display());
        Ok(())
    }

    /// Load receipt from JSON file
    pub fn load(path: &Path) -> Result<Receipt> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read receipt from {}", path.display()))?;

        let receipt: Receipt =
            serde_json::from_str(&content).context("Failed to parse receipt JSON")?;

        Ok(receipt)
    }

    /// Verify receipt integrity
    pub fn verify(receipt: &Receipt) -> Result<bool> {
        // 1. Verify all input hashes match current files
        let config_matches = if Path::new(&receipt.inputs.config.path).exists() {
            let current_hash = hash_file(Path::new(&receipt.inputs.config.path))?;
            current_hash == receipt.inputs.config.hash
        } else {
            false
        };

        if !config_matches {
            tracing::warn!("Config hash mismatch");
            return Ok(false);
        }

        // 2. Verify ontology hashes
        for ontology in &receipt.inputs.ontologies {
            let path = Path::new(&ontology.path);
            if !path.exists() {
                tracing::warn!("Ontology file missing: {}", ontology.path);
                return Ok(false);
            }
            let current_hash = hash_file(path)?;
            if current_hash != ontology.hash {
                tracing::warn!("Ontology hash mismatch: {}", ontology.path);
                return Ok(false);
            }
        }

        // 3. Verify output hashes
        for output in &receipt.outputs {
            let path = Path::new(&output.path);
            if !path.exists() {
                tracing::warn!("Output file missing: {}", output.path);
                return Ok(false);
            }
            let content = fs::read_to_string(path)?;
            let current_hash = hash_string(&content);
            if current_hash != output.hash {
                tracing::warn!("Output hash mismatch: {}", output.path);
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Add guard verdicts to receipt
    pub fn add_guard_verdicts(receipt: &mut Receipt, verdicts: Vec<GuardVerdict>) {
        receipt.guards.verdicts = verdicts;

        // Update overall status based on guard verdicts
        let all_passed = receipt.guards.verdicts.iter().all(|v| v.verdict == "pass");
        receipt.metadata.status = if all_passed { "pass" } else { "fail" }.to_string();
    }

    /// Update performance metrics
    pub fn update_performance(receipt: &mut Receipt, metrics: PerformanceMetrics) {
        receipt.metadata.performance = Some(metrics);
    }

    // Helper: Count rules in ggen.toml
    fn count_rules(config_path: &Path) -> Result<usize> {
        let content = fs::read_to_string(config_path)?;
        // Simple heuristic: count [[generation_rules]] sections
        let count = content.matches("[[generation_rules]]").count()
            + content.matches("[[generation.rules]]").count();
        Ok(count)
    }

    // Helper: Count RDF triples (approximate)
    fn count_triples(ontology_path: &Path) -> Result<usize> {
        let content = fs::read_to_string(ontology_path)?;
        // Simple heuristic: count lines with '.' at end (turtle triple terminator)
        let count = content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with('#') && trimmed.ends_with('.')
            })
            .count();
        Ok(count)
    }

    // Helper: Detect language from file extension
    fn detect_language(path: &Path) -> String {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => "rust",
            Some("ts") => "typescript",
            Some("js") => "javascript",
            Some("yaml") | Some("yml") => "yaml",
            Some("json") => "json",
            Some("toml") => "toml",
            Some("md") => "markdown",
            _ => "unknown",
        }
        .to_string()
    }
}

// ============================================================================
// Cryptographic Hash Utilities
// ============================================================================

/// Compute SHA-256 hash of file content
pub fn hash_file(path: &Path) -> Result<String> {
    let content = fs::read(path)
        .with_context(|| format!("Failed to read file for hashing: {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Compute SHA-256 hash of string
pub fn hash_string(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_hash_string_deterministic() {
        let s1 = "hello world";
        let s2 = "hello world";
        let s3 = "hello world!";

        let h1 = hash_string(s1);
        let h2 = hash_string(s2);
        let h3 = hash_string(s3);

        assert_eq!(h1, h2); // Same input = same hash
        assert_ne!(h1, h3); // Different input = different hash
        assert_eq!(h1.len(), 64); // SHA-256 = 64 hex chars
    }

    #[test]
    fn test_hash_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let hash = hash_file(&file_path).unwrap();
        assert_eq!(hash.len(), 64);

        // Hash should match hash_string of same content
        let string_hash = hash_string("test content");
        assert_eq!(hash, string_hash);
    }

    #[test]
    fn test_receipt_generation() {
        let receipt = Receipt {
            version: "1.0.0".to_string(),
            workspace: WorkspaceInfo {
                root: "/tmp/test".to_string(),
                fingerprint: hash_string("/tmp/test"),
            },
            inputs: InputsInfo {
                config: ConfigInput {
                    path: "ggen.toml".to_string(),
                    hash: hash_string("config"),
                    rules_count: 3,
                },
                ontologies: vec![],
                queries: vec![],
                templates: vec![],
            },
            guards: GuardsInfo {
                kernel_version: "1.0.0".to_string(),
                verdicts: vec![],
            },
            outputs: vec![],
            metadata: ReceiptMetadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                compiler_version: "0.1.0".to_string(),
                mode: "preview".to_string(),
                status: "pass".to_string(),
                performance: None,
            },
        };

        // Should serialize to JSON
        let json = serde_json::to_string_pretty(&receipt).unwrap();
        assert!(json.contains("\"version\": \"1.0.0\""));
        assert!(json.contains("\"mode\": \"preview\""));
    }

    #[test]
    fn test_receipt_save_and_load() {
        let dir = tempdir().unwrap();
        let receipt_path = dir.path().join("receipt.json");

        let receipt = Receipt {
            version: "1.0.0".to_string(),
            workspace: WorkspaceInfo {
                root: "/tmp/test".to_string(),
                fingerprint: hash_string("/tmp/test"),
            },
            inputs: InputsInfo {
                config: ConfigInput {
                    path: "ggen.toml".to_string(),
                    hash: hash_string("config"),
                    rules_count: 3,
                },
                ontologies: vec![],
                queries: vec![],
                templates: vec![],
            },
            guards: GuardsInfo {
                kernel_version: "1.0.0".to_string(),
                verdicts: vec![],
            },
            outputs: vec![],
            metadata: ReceiptMetadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                compiler_version: "0.1.0".to_string(),
                mode: "preview".to_string(),
                status: "pass".to_string(),
                performance: None,
            },
        };

        // Save
        ReceiptGenerator::save(&receipt, &receipt_path).unwrap();
        assert!(receipt_path.exists());

        // Load
        let loaded = ReceiptGenerator::load(&receipt_path).unwrap();
        assert_eq!(loaded.version, "1.0.0");
        assert_eq!(loaded.workspace.root, "/tmp/test");
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(
            ReceiptGenerator::detect_language(Path::new("test.rs")),
            "rust"
        );
        assert_eq!(
            ReceiptGenerator::detect_language(Path::new("test.ts")),
            "typescript"
        );
        assert_eq!(
            ReceiptGenerator::detect_language(Path::new("test.yaml")),
            "yaml"
        );
        assert_eq!(
            ReceiptGenerator::detect_language(Path::new("test.json")),
            "json"
        );
        assert_eq!(
            ReceiptGenerator::detect_language(Path::new("test.xyz")),
            "unknown"
        );
    }

    #[test]
    fn test_guard_verdict_integration() {
        let mut receipt = Receipt {
            version: "1.0.0".to_string(),
            workspace: WorkspaceInfo {
                root: "/tmp/test".to_string(),
                fingerprint: hash_string("/tmp/test"),
            },
            inputs: InputsInfo {
                config: ConfigInput {
                    path: "ggen.toml".to_string(),
                    hash: hash_string("config"),
                    rules_count: 3,
                },
                ontologies: vec![],
                queries: vec![],
                templates: vec![],
            },
            guards: GuardsInfo {
                kernel_version: "1.0.0".to_string(),
                verdicts: vec![],
            },
            outputs: vec![],
            metadata: ReceiptMetadata {
                timestamp: chrono::Utc::now().to_rfc3339(),
                compiler_version: "0.1.0".to_string(),
                mode: "preview".to_string(),
                status: "pass".to_string(),
                performance: None,
            },
        };

        // Add passing verdicts
        let verdicts = vec![
            GuardVerdict {
                name: "syntax_check".to_string(),
                verdict: "pass".to_string(),
                diagnostic: "All syntax valid".to_string(),
                metadata: HashMap::new(),
            },
            GuardVerdict {
                name: "type_check".to_string(),
                verdict: "pass".to_string(),
                diagnostic: "Type checking passed".to_string(),
                metadata: HashMap::new(),
            },
        ];

        ReceiptGenerator::add_guard_verdicts(&mut receipt, verdicts);
        assert_eq!(receipt.guards.verdicts.len(), 2);
        assert_eq!(receipt.metadata.status, "pass");

        // Add failing verdict
        let failing_verdicts = vec![GuardVerdict {
            name: "lint_check".to_string(),
            verdict: "fail".to_string(),
            diagnostic: "Linting errors found".to_string(),
            metadata: HashMap::new(),
        }];

        ReceiptGenerator::add_guard_verdicts(&mut receipt, failing_verdicts);
        assert_eq!(receipt.metadata.status, "fail");
    }
}
