//! Example: Ontology Validation and Consistency Checking
//!
//! This example demonstrates how to use the ontology consistency checking system
//! to validate RDF ontologies before code generation.
//!
//! Run with:
//! ```bash
//! cargo run --example ontology_validation
//! ```

use anyhow::Result;
use oxigraph::io::GraphFormat;
use oxigraph::store::Store;
use spreadsheet_mcp::ontology::{
    ConsistencyChecker, HashVerifier, NamespaceManager, SchemaValidator,
};

fn main() -> Result<()> {
    println!("=== Ontology Validation Example ===\n");

    // Create an RDF store and load the MCP domain ontology
    let store = Store::new()?;

    println!("Loading ontology...");
    match store.load_from_file("ontology/mcp-domain.ttl", GraphFormat::Turtle) {
        Ok(_) => println!("✓ Ontology loaded successfully\n"),
        Err(e) => {
            eprintln!("✗ Failed to load ontology: {}", e);
            return Ok(());
        }
    }

    // 1. Run Consistency Checks
    println!("=== Running Consistency Checks ===");
    let consistency_checker = ConsistencyChecker::new(store.clone());
    let consistency_report = consistency_checker.check_all();

    println!("\nConsistency Report:");
    println!(
        "  Status: {}",
        if consistency_report.valid {
            "✓ Valid"
        } else {
            "✗ Invalid"
        }
    );
    println!("\nStatistics:");
    println!(
        "  - Total triples: {}",
        consistency_report.stats.total_triples
    );
    println!(
        "  - Total classes: {}",
        consistency_report.stats.total_classes
    );
    println!(
        "  - Total properties: {}",
        consistency_report.stats.total_properties
    );
    println!(
        "  - Total individuals: {}",
        consistency_report.stats.total_individuals
    );
    println!(
        "  - Max hierarchy depth: {}",
        consistency_report.stats.max_hierarchy_depth
    );

    if !consistency_report.errors.is_empty() {
        println!("\n❌ Errors found:");
        for (i, error) in consistency_report.errors.iter().enumerate() {
            println!("  {}. {}", i + 1, error);
        }
    }

    if !consistency_report.warnings.is_empty() {
        println!("\n⚠ Warnings:");
        for (i, warning) in consistency_report.warnings.iter().enumerate() {
            println!("  {}. {}", i + 1, warning);
        }
    }

    // 2. Run Schema Validation
    println!("\n=== Running Schema Validation ===");
    let schema_validator = SchemaValidator::new(store.clone());
    let schema_report = schema_validator.validate_all();

    println!("\nSchema Validation Report:");
    println!(
        "  Status: {}",
        if schema_report.valid {
            "✓ Valid"
        } else {
            "✗ Invalid"
        }
    );

    if !schema_report.errors.is_empty() {
        println!("\n❌ Schema errors found:");
        for (i, error) in schema_report.errors.iter().enumerate() {
            println!("  {}. {}", i + 1, error);
        }
    }

    if !schema_report.warnings.is_empty() {
        println!("\n⚠ Schema warnings:");
        for (i, warning) in schema_report.warnings.iter().enumerate() {
            println!("  {}. {}", i + 1, warning);
        }
    }

    // 3. Namespace Management
    println!("\n=== Namespace Management ===");
    let mut ns_manager = NamespaceManager::new();

    // Register custom namespace
    ns_manager.register("mcp", "http://ggen-mcp.dev/ontology/mcp#")?;

    println!("\nRegistered namespaces:");
    for (prefix, uri) in ns_manager.all() {
        println!("  {}: {}", prefix, uri);
    }

    // Demonstrate QName expansion
    let qname = "mcp:Tool";
    let expanded = ns_manager.expand(qname)?;
    println!("\nQName expansion:");
    println!("  {} -> {}", qname, expanded);

    // Demonstrate URI compaction
    let uri = "http://ggen-mcp.dev/ontology/mcp#Resource";
    let compacted = ns_manager.compact(uri);
    println!("\nURI compaction:");
    println!("  {} -> {}", uri, compacted);

    // 4. Hash Verification
    println!("\n=== Hash Verification ===");
    let hash_verifier = HashVerifier::new(store);
    let computed_hash = hash_verifier.compute_hash()?;

    println!("\nOntology hash (SHA-256):");
    println!("  {}", computed_hash);

    // Check if there's a stored hash
    match hash_verifier.get_ontology_hash()? {
        Some(stored_hash) => {
            println!("\nStored hash found:");
            println!("  {}", stored_hash);

            if stored_hash == computed_hash {
                println!("  ✓ Hash verification passed - ontology is unchanged");
            } else {
                println!("  ✗ Hash mismatch - ontology has been modified!");
                println!("    Expected: {}", stored_hash);
                println!("    Got:      {}", computed_hash);
            }
        }
        None => {
            println!("\nNo stored hash found. This is the first validation.");
            println!("To store this hash for future verification, add it to your ontology:");
            println!(
                "\n  ggen:ontology ggen:ontologyHash \"{}\" .",
                computed_hash
            );
        }
    }

    // 5. Final Summary
    println!("\n=== Validation Summary ===");
    if consistency_report.valid && schema_report.valid {
        println!("✓ All validations passed!");
        println!("  The ontology is ready for code generation.");
    } else {
        println!("✗ Validation failed");
        println!("  Please fix the errors above before generating code.");
        return Err(anyhow::anyhow!("Validation failed"));
    }

    Ok(())
}
