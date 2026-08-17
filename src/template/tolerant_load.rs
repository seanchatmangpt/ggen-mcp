//! Tolerant template directory loading
//!
//! `Tera::new(glob)` is *eager*: a single unparseable template in the directory
//! aborts the whole load, so no template in that directory can be rendered or
//! even inspected. For a security-surface validator that is the wrong failure
//! mode — it collapses "template X is malformed" into "the validator does not
//! exist", which means the malformed template can never be reported *as* a
//! syntax error against the template that caused it.
//!
//! This module loads a directory template-by-template instead. Templates that
//! parse are registered; templates that do not are recorded in a per-template
//! error map so the failure is surfaced when *that* template is validated or
//! rendered — fail-closed per template, not fail-closed for the directory.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use tera::Tera;
use walkdir::WalkDir;

/// Result of a tolerant directory load
pub struct TolerantLoad {
    /// Tera instance containing every template that parsed successfully
    pub tera: Tera,
    /// Template name -> parse error, for every template that failed to parse
    pub errors: HashMap<String, String>,
}

impl TolerantLoad {
    /// Look up the recorded parse error for a template, if any
    pub fn error_for(&self, template_name: &str) -> Option<&String> {
        self.errors.get(template_name)
    }
}

/// Format a `tera::Error` including its full source chain.
///
/// `Display` on the top-level error is only "Failed to parse <path>"; the
/// actual syntax diagnostic lives in the source chain.
fn format_tera_error(err: &tera::Error) -> String {
    let mut msg = err.to_string();
    let mut source: Option<&(dyn std::error::Error + 'static)> = std::error::Error::source(err);
    while let Some(e) = source {
        msg.push_str(": ");
        msg.push_str(&e.to_string());
        source = e.source();
    }
    msg
}

/// Load every file under `dir` into a Tera instance, one template at a time.
///
/// Template names are the file paths relative to `dir`, using `/` separators —
/// identical to the names `Tera::new("<dir>/**/*")` would assign.
///
/// If `tera_extension_only` is true, only files ending in `.tera` are loaded
/// (matching a `**/*.tera` glob); otherwise every file is loaded.
pub fn load_dir_tolerant(dir: impl AsRef<Path>, tera_extension_only: bool) -> Result<TolerantLoad> {
    let dir = dir.as_ref();
    let mut tera = Tera::default();
    let mut errors = HashMap::new();

    for entry in WalkDir::new(dir).follow_links(false) {
        let entry = entry.with_context(|| format!("failed to walk template dir {:?}", dir))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if tera_extension_only && path.extension().and_then(|e| e.to_str()) != Some("tera") {
            continue;
        }

        let rel = path.strip_prefix(dir).unwrap_or(path);
        let name = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                // Non-UTF8 or unreadable file: record it, do not abort the load.
                errors.insert(name, format!("failed to read template: {}", e));
                continue;
            }
        };

        if let Err(e) = tera.add_raw_template(&name, &content) {
            errors.insert(name, format_tera_error(&e));
        }
    }

    Ok(TolerantLoad { tera, errors })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn one_bad_template_does_not_block_the_good_ones() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("good.rs.tera"), "hello {{ name }}").unwrap();
        std::fs::write(dir.path().join("bad.rs.tera"), "oops {{ name } {").unwrap();

        let loaded = load_dir_tolerant(dir.path(), true).unwrap();

        // The good template is usable.
        let mut ctx = tera::Context::new();
        ctx.insert("name", "world");
        assert_eq!(loaded.tera.render("good.rs.tera", &ctx).unwrap(), "hello world");

        // The bad template is recorded against its own name.
        let err = loaded.error_for("bad.rs.tera").expect("bad template error recorded");
        assert!(err.to_lowercase().contains("parse"), "{}", err);
        assert!(loaded.error_for("good.rs.tera").is_none());
    }
}
