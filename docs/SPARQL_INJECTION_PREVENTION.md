# SPARQL Injection Prevention Guide

## Table of Contents

1. [Introduction](#introduction)
2. [Common Attack Patterns](#common-attack-patterns)
3. [Security Components](#security-components)
4. [Safe Query Construction](#safe-query-construction)
5. [Integration Guide](#integration-guide)
6. [Testing Strategies](#testing-strategies)
7. [Quick Reference](#quick-reference)
8. [Best Practices](#best-practices)

## Introduction

SPARQL injection is a security vulnerability that occurs when untrusted user input is incorporated into SPARQL queries without proper validation or escaping. Like SQL injection, it can allow attackers to:

- Access unauthorized data
- Modify or delete RDF triples
- Execute arbitrary SPARQL queries
- Bypass application logic

This guide documents the comprehensive SPARQL injection prevention mechanisms implemented in the `ggen-mcp` system.

## Common Attack Patterns

### 1. Comment Injection

**Attack Vector:**
```sparql
# User input: admin' # rest is commented
SELECT ?user WHERE {
  ?user :name "admin' # rest is commented" .
  ?user :role ?role
}
```

**Result:** The rest of the query is commented out, potentially bypassing authentication checks.

**Prevention:** The `SparqlSanitizer` detects and blocks both `#` and `//` comment syntax.

### 2. Union-Based Injection

**Attack Vector:**
```sparql
# User input: ' } UNION { ?s ?p ?o }
SELECT ?data WHERE {
  ?resource :value "' } UNION { ?s ?p ?o }" .
}
```

**Result:** Attacker can retrieve all triples from the dataset.

**Prevention:** The `SparqlSanitizer` detects `UNION` keywords and query structure manipulation (braces).

### 3. Filter Manipulation

**Attack Vector:**
```sparql
# User input: ' } FILTER(1=1) { '
SELECT ?data WHERE {
  ?resource :value "' } FILTER(1=1) { '" .
  FILTER(?sensitive = true)
}
```

**Result:** Bypasses security filters with tautology.

**Prevention:** Detection of `FILTER`, `OPTIONAL`, and other SPARQL keywords in user input.

### 4. Destructive Queries

**Attack Vector:**
```sparql
# User input: '; DROP GRAPH <http://example.org/data>
INSERT DATA {
  :resource :value "'; DROP GRAPH <http://example.org/data>" .
}
```

**Result:** Data loss through graph deletion.

**Prevention:** Blocking of `DROP`, `DELETE`, `CLEAR`, `INSERT`, and other update operations.

### 5. IRI Injection

**Attack Vector:**
```sparql
# User input: http://evil.com> } { <http://sensitive.internal/data
SELECT ?data WHERE {
  ?s :resource <http://evil.com> } { <http://sensitive.internal/data> .
}
```

**Result:** Query structure manipulation through IRI boundaries.

**Prevention:** IRI validation and escaping of special characters.

## Security Components

### SparqlSanitizer

Provides low-level escaping and validation functions.

```rust
use spreadsheet_mcp::sparql::SparqlSanitizer;

// Escape string literals
let safe_string = SparqlSanitizer::escape_string("O'Reilly")?;
// Result: "O\\'Reilly"

// Escape IRIs
let safe_iri = SparqlSanitizer::escape_iri("http://example.org/resource")?;

// Validate and escape numbers
let safe_number = SparqlSanitizer::escape_number("42")?;
```

**Security Features:**
- Escapes special characters: `\`, `"`, `'`, `\n`, `\r`, `\t`
- Detects comment injection: `#`, `//`
- Blocks malicious patterns: `UNION`, `FILTER`, `INSERT`, `DELETE`, etc.
- Validates query structure (no unmatched braces)

### QueryBuilder

Type-safe query construction with the builder pattern.

```rust
use spreadsheet_mcp::sparql::QueryBuilder;

let query = QueryBuilder::select()
    .prefix("foaf", "http://xmlns.com/foaf/0.1/")
    .variable("?person")
    .variable("?name")
    .where_clause("?person a foaf:Person")
    .where_clause("?person foaf:name ?name")
    .filter("(?age > 18)")
    .order_by("?name")
    .limit(10)
    .build()?;
```

**Security Features:**
- Validates variables before adding to query
- Rejects WHERE clauses with destructive keywords
- Validates IRIs in PREFIX declarations
- Prevents query structure manipulation

### VariableValidator

Validates SPARQL variable names according to W3C specifications.

```rust
use spreadsheet_mcp::sparql::VariableValidator;

// Valid variables
VariableValidator::validate("?person")?;   // ✓
VariableValidator::validate("$name_123")?; // ✓

// Invalid variables
VariableValidator::validate("person")?;    // ✗ Missing ? or $
VariableValidator::validate("?my var")?;   // ✗ Contains space
VariableValidator::validate("?s")?;        // ✗ Reserved keyword
```

**Validation Rules:**
- Must start with `?` or `$`
- Can contain alphanumeric characters and underscore
- Maximum length: 128 characters
- Cannot be a reserved keyword (`?s`, `?p`, `?o`, etc.)

### SafeLiteralBuilder

Type-safe construction of RDF literals with automatic escaping.

```rust
use spreadsheet_mcp::sparql::SafeLiteralBuilder;

// String literals
let name = SafeLiteralBuilder::string("John O'Brien").build();
// Result: "John O\'Brien"

// Typed literals
let age = SafeLiteralBuilder::integer(42).build();
// Result: "42"^^<http://www.w3.org/2001/XMLSchema#integer>

let price = SafeLiteralBuilder::decimal(19.99).build();
// Result: "19.99"^^<http://www.w3.org/2001/XMLSchema#decimal>

let active = SafeLiteralBuilder::boolean(true).build();
// Result: "true"^^<http://www.w3.org/2001/XMLSchema#boolean>

let timestamp = SafeLiteralBuilder::datetime("2024-01-20T12:00:00Z").build();
// Result: "2024-01-20T12:00:00Z"^^<http://www.w3.org/2001/XMLSchema#dateTime>

// Language-tagged literals
let greeting = SafeLiteralBuilder::string("Bonjour").language("fr").build();
// Result: "Bonjour"@fr

// Custom datatypes
let custom = SafeLiteralBuilder::string("value")
    .with_datatype("http://example.org/customType")
    .build();
```

**Security Features:**
- Automatic string escaping via `SparqlSanitizer`
- Type-safe literal construction
- Language tag validation (BCP 47)
- Prevention of quote escaping attacks

### IriValidator

Validates IRIs and URIs according to RFC 3987.

```rust
use spreadsheet_mcp::sparql::IriValidator;

// Valid IRIs
IriValidator::validate("http://example.org")?;              // ✓
IriValidator::validate("https://example.org/path?q=v")?;    // ✓
IriValidator::validate("urn:isbn:0451450523")?;             // ✓
IriValidator::validate("mailto:user@example.org")?;         // ✓

// Invalid IRIs
IriValidator::validate("javascript:alert(1)")?;             // ✗ Invalid scheme
IriValidator::validate("http://example.org/my resource")?;  // ✗ Contains space
IriValidator::validate("http://example.org/<script>")?;     // ✗ Angle brackets

// Absolute URI validation
IriValidator::require_absolute("http://example.org")?;      // ✓
IriValidator::require_absolute("relative/path")?;           // ✗
```

**Validation Rules:**
- Valid URI scheme: `http`, `https`, `urn`, `ftp`, `file`, `mailto`, `tel`, `data`
- No spaces or special characters
- No angle brackets (would break SPARQL syntax)
- Optional requirement for absolute URIs

## Safe Query Construction

### Example 1: Basic SELECT Query with User Input

```rust
use spreadsheet_mcp::sparql::{QueryBuilder, SafeLiteralBuilder};

fn search_person_by_name(user_input: &str) -> Result<String> {
    // User input is automatically escaped by SafeLiteralBuilder
    let name_literal = SafeLiteralBuilder::string(user_input).build();
    
    let query = QueryBuilder::select()
        .prefix("foaf", "http://xmlns.com/foaf/0.1/")
        .variable("?person")
        .variable("?email")
        .where_clause("?person a foaf:Person")
        .where_clause(&format!("?person foaf:name {}", name_literal))
        .where_clause("?person foaf:mbox ?email")
        .limit(10)
        .build()?;
    
    Ok(query)
}

// Usage
let query = search_person_by_name("John O'Brien")?;
// Safe query with escaped apostrophe
```

### Example 2: Parameterized ASK Query

```rust
use spreadsheet_mcp::sparql::{QueryBuilder, SafeLiteralBuilder, IriValidator};

fn check_permission(user_id: &str, resource_iri: &str) -> Result<String> {
    // Validate IRI before use
    IriValidator::require_absolute(resource_iri)?;
    
    let user_lit = SafeLiteralBuilder::string(user_id).build();
    
    let query = QueryBuilder::ask()
        .prefix("acl", "http://www.w3.org/ns/auth/acl#")
        .where_clause(&format!("?user :id {}", user_lit))
        .where_clause(&format!("?user acl:accessTo <{}>", resource_iri))
        .where_clause("?user acl:mode acl:Read")
        .build()?;
    
    Ok(query)
}
```

### Example 3: Complex Query with Multiple Parameters

```rust
use spreadsheet_mcp::sparql::{QueryBuilder, SafeLiteralBuilder, VariableValidator};

fn search_products(
    category: &str,
    min_price: f64,
    max_price: f64,
    sort_by: &str,
) -> Result<String> {
    // Validate sort variable
    let sort_var = format!("?{}", sort_by);
    VariableValidator::validate(&sort_var)?;
    
    let category_lit = SafeLiteralBuilder::string(category).build();
    let min_lit = SafeLiteralBuilder::decimal(min_price).build();
    let max_lit = SafeLiteralBuilder::decimal(max_price).build();
    
    let query = QueryBuilder::select()
        .distinct()
        .prefix("shop", "http://example.org/shop#")
        .variable("?product")
        .variable("?name")
        .variable("?price")
        .where_clause("?product a shop:Product")
        .where_clause(&format!("?product shop:category {}", category_lit))
        .where_clause("?product shop:name ?name")
        .where_clause("?product shop:price ?price")
        .filter(&format!("(?price >= {})", min_lit))
        .filter(&format!("(?price <= {})", max_lit))
        .order_by(&sort_var)
        .limit(20)
        .build()?;
    
    Ok(query)
}
```

### Example 4: CONSTRUCT Query for Data Transformation

```rust
use spreadsheet_mcp::sparql::{QueryBuilder, SafeLiteralBuilder};

fn transform_legacy_data(source_graph: &str) -> Result<String> {
    IriValidator::validate(source_graph)?;
    
    let query = QueryBuilder::construct()
        .prefix("old", "http://example.org/old#")
        .prefix("new", "http://example.org/new#")
        .where_clause("?person old:fullName ?name")
        .where_clause("?person old:emailAddress ?email")
        .where_clause("?person old:birthYear ?year")
        .build()?;
    
    Ok(query)
}
```

## Integration Guide

### Step 1: Add Module Import

```rust
use spreadsheet_mcp::sparql::{
    QueryBuilder,
    SafeLiteralBuilder,
    SparqlSanitizer,
    IriValidator,
    VariableValidator,
};
```

### Step 2: Replace Manual Query Construction

**Before (Unsafe):**
```rust
fn build_query(user_input: &str) -> String {
    format!(
        "SELECT ?person WHERE {{ ?person :name \"{}\" }}",
        user_input  // ⚠️ VULNERABLE TO INJECTION
    )
}
```

**After (Safe):**
```rust
fn build_query(user_input: &str) -> Result<String> {
    let name_lit = SafeLiteralBuilder::string(user_input).build();
    
    QueryBuilder::select()
        .variable("?person")
        .where_clause("?person :name " + &name_lit)
        .build()
}
```

### Step 3: Validate All External Input

```rust
fn process_user_query(
    variable_name: &str,
    iri: &str,
    literal_value: &str,
) -> Result<String> {
    // Validate variable
    let var = format!("?{}", variable_name);
    VariableValidator::validate(&var)?;
    
    // Validate IRI
    IriValidator::require_absolute(iri)?;
    
    // Build safe literal
    let lit = SafeLiteralBuilder::string(literal_value).build();
    
    // Construct query
    QueryBuilder::select()
        .variable(&var)
        .where_clause(&format!("{} :value {}", var, lit))
        .build()
}
```

### Step 4: Handle Errors Appropriately

```rust
use spreadsheet_mcp::sparql::SparqlSecurityError;

fn execute_user_query(input: &str) -> Result<String, String> {
    match build_safe_query(input) {
        Ok(query) => Ok(query),
        Err(SparqlSecurityError::CommentInjection) => {
            Err("Invalid input: comment characters not allowed".to_string())
        }
        Err(SparqlSecurityError::MaliciousPattern(pattern)) => {
            Err(format!("Invalid input: contains restricted keyword {}", pattern))
        }
        Err(e) => Err(format!("Query validation failed: {}", e)),
    }
}
```

## Testing Strategies

### Unit Testing

Test individual components with malicious inputs:

```rust
#[test]
fn test_sanitizer_blocks_union_injection() {
    let malicious = "' } UNION { ?s ?p ?o }";
    let result = SparqlSanitizer::escape_string(malicious);
    assert!(result.is_err());
}

#[test]
fn test_variable_validator_rejects_invalid() {
    assert!(VariableValidator::validate("?my var").is_err());
    assert!(VariableValidator::validate("noprefix").is_err());
}
```

### Integration Testing

Test complete query construction with realistic scenarios:

```rust
#[test]
fn test_safe_user_search() {
    let malicious_input = "admin' # bypass";
    
    let result = search_person_by_name(malicious_input);
    
    // Should either succeed with escaped input or fail validation
    match result {
        Ok(query) => {
            assert!(!query.contains('#'));
            assert!(query.contains("\\'"));
        }
        Err(_) => {
            // Acceptable: rejected malicious input
        }
    }
}
```

### Penetration Testing

Use the comprehensive test suite in `tests/sparql_injection_tests.rs`:

```bash
cargo test sparql_injection
```

### Fuzzing

Create a fuzzing test to discover edge cases:

```rust
use arbitrary::Arbitrary;

#[cfg(test)]
mod fuzz_tests {
    use super::*;
    
    #[test]
    fn fuzz_sanitizer() {
        // Use a fuzzing library like cargo-fuzz
        // or quickcheck to generate random inputs
        for _ in 0..10000 {
            let random_input = generate_random_string();
            let _ = SparqlSanitizer::escape_string(&random_input);
            // Should never panic
        }
    }
}
```

## Quick Reference

### Common Patterns

| Task | Solution |
|------|----------|
| Escape user string | `SafeLiteralBuilder::string(input).build()` |
| Build SELECT query | `QueryBuilder::select().variable("?x").where_clause("...").build()?` |
| Validate IRI | `IriValidator::validate(iri)?` |
| Validate variable | `VariableValidator::validate("?var")?` |
| Create typed literal | `SafeLiteralBuilder::integer(42).build()` |
| Add language tag | `SafeLiteralBuilder::string("text").language("en").build()` |

### Error Handling

```rust
match result {
    Err(SparqlSecurityError::InvalidIri(msg)) => { /* ... */ }
    Err(SparqlSecurityError::InvalidVariable(msg)) => { /* ... */ }
    Err(SparqlSecurityError::MaliciousPattern(pattern)) => { /* ... */ }
    Err(SparqlSecurityError::CommentInjection) => { /* ... */ }
    Err(SparqlSecurityError::StructureManipulation) => { /* ... */ }
    Err(SparqlSecurityError::InvalidScheme(scheme)) => { /* ... */ }
    // ... other errors
}
```

### Security Checklist

- [ ] Never concatenate user input directly into queries
- [ ] Always use `SafeLiteralBuilder` for string literals
- [ ] Validate variables with `VariableValidator`
- [ ] Validate IRIs with `IriValidator`
- [ ] Use `QueryBuilder` for query construction
- [ ] Handle all security errors appropriately
- [ ] Test with malicious inputs
- [ ] Review code for manual SPARQL string construction
- [ ] Enable audit logging for SPARQL operations
- [ ] Monitor for unusual query patterns

## Best Practices

### 1. Defense in Depth

Use multiple layers of security:

```rust
fn secure_query(user_var: &str, user_iri: &str, user_value: &str) -> Result<String> {
    // Layer 1: Input validation
    let var = format!("?{}", user_var);
    VariableValidator::validate(&var)?;
    
    // Layer 2: IRI validation
    IriValidator::require_absolute(user_iri)?;
    
    // Layer 3: Safe literal building
    let lit = SafeLiteralBuilder::string(user_value).build();
    
    // Layer 4: Type-safe query construction
    QueryBuilder::select()
        .variable(&var)
        .where_clause(&format!("{} <{}> {}", var, user_iri, lit))
        .build()
}
```

### 2. Fail Securely

When validation fails, reject the request completely:

```rust
fn process_request(input: &str) -> Result<Response, Error> {
    let query = build_safe_query(input)
        .map_err(|e| Error::SecurityViolation(e))?;
    
    // Only proceed if validation succeeded
    execute_query(query)
}
```

### 3. Principle of Least Privilege

Grant only necessary SPARQL permissions:

```rust
// Good: Read-only queries
fn search_data(input: &str) -> Result<String> {
    QueryBuilder::select() /* ... */
}

// Bad: Allowing UPDATE queries from user input
fn dangerous_update(input: &str) -> Result<String> {
    // Don't do this!
    format!("INSERT DATA {{ {} }}", input)
}
```

### 4. Audit and Monitor

Log all SPARQL queries for security review:

```rust
use tracing::info;

fn execute_query(query: &str) -> Result<Response> {
    info!(
        query = query,
        "executing SPARQL query"
    );
    
    // Execute query...
}
```

### 5. Regular Security Reviews

- Review code for manual SPARQL string construction
- Update to latest security patches
- Run penetration tests regularly
- Monitor for new SPARQL injection techniques

## Additional Resources

- [SPARQL 1.1 Query Language Specification](https://www.w3.org/TR/sparql11-query/)
- [OWASP: Injection Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Injection_Prevention_Cheat_Sheet.html)
- [RFC 3987: Internationalized Resource Identifiers (IRIs)](https://www.rfc-editor.org/rfc/rfc3987)
- [RDF 1.1 Concepts and Abstract Syntax](https://www.w3.org/TR/rdf11-concepts/)

## Support

For questions or security concerns, please:

1. Review this documentation
2. Check the test suite in `tests/sparql_injection_tests.rs`
3. Examine the module documentation in `src/sparql/injection_prevention.rs`
4. Open an issue on GitHub (for non-security bugs)
5. Contact the security team directly (for security vulnerabilities)

---

**Last Updated:** 2024-01-20
**Version:** 1.0.0
