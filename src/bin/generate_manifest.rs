//! Generate tool manifest for OpenAPI schema validation.
//!
//! Usage:
//!   cargo run --bin generate_manifest > ggen.tools.json
//!   cargo run --bin generate_manifest --pretty  # Pretty-printed output
//!
//! Output: JSON manifest with tool schemas, version, and breaking change hash.

use spreadsheet_mcp::tools::manifest::ManifestGenerator;
use std::env;

fn main() {
    let pretty = env::args().any(|arg| arg == "--pretty");

    let manifest = ManifestGenerator::generate();
    let json = if pretty {
        serde_json::to_string_pretty(&manifest).expect("Failed to serialize manifest")
    } else {
        serde_json::to_string(&manifest).expect("Failed to serialize manifest")
    };

    println!("{}", json);
}
