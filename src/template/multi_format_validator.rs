//! Multi-Format Validation Module
//!
//! Provides syntax validation for TypeScript, YAML, JSON, and OpenAPI specifications
//! using pattern-based validation and existing serde parsers. No external compilers required.
//!
//! ## Design Principles (Poka-Yoke)
//! - Fail-fast on syntax errors
//! - Detailed error messages with line numbers
//! - Suggestions for common mistakes
//! - Pattern-based validation (regex + structural checks)
//!
//! ## Supported Formats
//! - **TypeScript**: Basic syntax checking via regex patterns
//! - **YAML**: Syntax validation via serde_yaml
//! - **JSON**: Syntax validation via serde_json
//! - **OpenAPI**: YAML + schema structure validation

use anyhow::{Context, Result};
use regex::Regex;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::HashSet;

use crate::codegen::validation::{ValidationReport, ValidationSeverity};

// =============================================================================
// TypeScript Validator (Pattern-Based)
// =============================================================================

/// Validates TypeScript syntax using pattern matching (no swc dependency)
pub struct TypeScriptValidator {
    /// Track seen identifiers to detect potential duplicates
    seen_identifiers: HashSet<String>,
}

impl TypeScriptValidator {
    pub fn new() -> Self {
        Self {
            seen_identifiers: HashSet::new(),
        }
    }

    /// Validate TypeScript code syntax
    pub fn validate(&mut self, code: &str, file_name: &str) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();

        // 1. Balanced braces/brackets/parens
        self.validate_balanced_delimiters(code, &mut report, file_name);

        // 2. Import/export syntax
        self.validate_imports_exports(code, &mut report, file_name);

        // 3. Valid identifiers
        self.validate_identifiers(code, &mut report, file_name);

        // 4. Common TypeScript errors
        self.validate_common_errors(code, &mut report, file_name);

        // 5. Interface/type declarations
        self.validate_type_declarations(code, &mut report, file_name);

        // 6. Function declarations
        self.validate_function_declarations(code, &mut report, file_name);

        Ok(report)
    }

    /// Validate balanced delimiters
    fn validate_balanced_delimiters(
        &self,
        code: &str,
        report: &mut ValidationReport,
        file_name: &str,
    ) {
        let mut brace_stack = Vec::new();
        let mut bracket_stack = Vec::new();
        let mut paren_stack = Vec::new();

        for (line_num, line) in code.lines().enumerate() {
            // Skip comments and strings (simplified)
            let cleaned = self.remove_strings_and_comments(line);

            for (char_idx, ch) in cleaned.chars().enumerate() {
                match ch {
                    '{' => brace_stack.push((line_num + 1, char_idx + 1)),
                    '}' => {
                        if brace_stack.pop().is_none() {
                            report.add_error(
                                "Unmatched closing brace '}'".to_string(),
                                Some(format!("{}:{}:{}", file_name, line_num + 1, char_idx + 1)),
                                Some("Check for matching opening brace '{'".to_string()),
                            );
                        }
                    }
                    '[' => bracket_stack.push((line_num + 1, char_idx + 1)),
                    ']' => {
                        if bracket_stack.pop().is_none() {
                            report.add_error(
                                "Unmatched closing bracket ']'".to_string(),
                                Some(format!("{}:{}:{}", file_name, line_num + 1, char_idx + 1)),
                                Some("Check for matching opening bracket '['".to_string()),
                            );
                        }
                    }
                    '(' => paren_stack.push((line_num + 1, char_idx + 1)),
                    ')' => {
                        if paren_stack.pop().is_none() {
                            report.add_error(
                                "Unmatched closing parenthesis ')'".to_string(),
                                Some(format!("{}:{}:{}", file_name, line_num + 1, char_idx + 1)),
                                Some("Check for matching opening parenthesis '('".to_string()),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        // Check for unclosed delimiters
        if let Some((line, col)) = brace_stack.first() {
            report.add_error(
                "Unclosed opening brace '{'".to_string(),
                Some(format!("{}:{}:{}", file_name, line, col)),
                Some("Add matching closing brace '}'".to_string()),
            );
        }
        if let Some((line, col)) = bracket_stack.first() {
            report.add_error(
                "Unclosed opening bracket '['".to_string(),
                Some(format!("{}:{}:{}", file_name, line, col)),
                Some("Add matching closing bracket ']'".to_string()),
            );
        }
        if let Some((line, col)) = paren_stack.first() {
            report.add_error(
                "Unclosed opening parenthesis '('".to_string(),
                Some(format!("{}:{}:{}", file_name, line, col)),
                Some("Add matching closing parenthesis ')'".to_string()),
            );
        }
    }

    /// Validate import/export statements
    fn validate_imports_exports(&self, code: &str, report: &mut ValidationReport, file_name: &str) {
        let import_regex = Regex::new(r"^\s*import\s+").unwrap();
        let export_regex = Regex::new(r"^\s*export\s+").unwrap();

        for (line_num, line) in code.lines().enumerate() {
            let trimmed = line.trim();

            // Check import syntax
            if import_regex.is_match(trimmed) {
                if !trimmed.contains("from") && !trimmed.ends_with(';') {
                    // Simple import without 'from' should end with semicolon
                    if !trimmed.contains('{') && !trimmed.contains('*') {
                        report.add_warning(
                            "Import statement may be incomplete".to_string(),
                            Some(format!("{}:{}", file_name, line_num + 1)),
                            Some(
                                "Import should have 'from' clause or be a type-only import"
                                    .to_string(),
                            ),
                        );
                    }
                }

                // Check for common mistakes
                if trimmed.contains("form ") {
                    report.add_error(
                        "Typo in import: 'form' should be 'from'".to_string(),
                        Some(format!("{}:{}", file_name, line_num + 1)),
                        Some("Change 'form' to 'from'".to_string()),
                    );
                }
            }

            // Check export syntax
            if export_regex.is_match(trimmed) {
                // Validate export structure
                if trimmed == "export" {
                    report.add_error(
                        "Incomplete export statement".to_string(),
                        Some(format!("{}:{}", file_name, line_num + 1)),
                        Some("Export must be followed by declaration or identifier".to_string()),
                    );
                }
            }
        }
    }

    /// Validate identifier naming
    fn validate_identifiers(&mut self, code: &str, report: &mut ValidationReport, file_name: &str) {
        // Regex for valid TypeScript identifiers
        let identifier_regex = Regex::new(r"\b([a-zA-Z_$][a-zA-Z0-9_$]*)\b").unwrap();
        let reserved_words = get_typescript_reserved_words();

        for (line_num, line) in code.lines().enumerate() {
            for cap in identifier_regex.captures_iter(line) {
                let identifier = &cap[1];

                // Check for reserved words used as identifiers
                if reserved_words.contains(identifier) {
                    // Only error if it looks like a variable declaration
                    if line.contains(&format!("const {}", identifier))
                        || line.contains(&format!("let {}", identifier))
                        || line.contains(&format!("var {}", identifier))
                    {
                        report.add_error(
                            format!(
                                "Reserved word '{}' cannot be used as identifier",
                                identifier
                            ),
                            Some(format!("{}:{}", file_name, line_num + 1)),
                            Some("Use a different identifier name".to_string()),
                        );
                    }
                }

                // Track interface/type/class names
                if line.contains(&format!("interface {}", identifier))
                    || line.contains(&format!("type {}", identifier))
                    || line.contains(&format!("class {}", identifier))
                {
                    if self.seen_identifiers.contains(identifier) {
                        report.add_warning(
                            format!("Duplicate type identifier: {}", identifier),
                            Some(format!("{}:{}", file_name, line_num + 1)),
                            Some(
                                "Consider using a unique name or extending existing type"
                                    .to_string(),
                            ),
                        );
                    } else {
                        self.seen_identifiers.insert(identifier.to_string());
                    }
                }
            }
        }
    }

    /// Validate common TypeScript errors
    fn validate_common_errors(&self, code: &str, report: &mut ValidationReport, file_name: &str) {
        for (line_num, line) in code.lines().enumerate() {
            let trimmed = line.trim();

            // Trailing commas in object literals (potential issue)
            if trimmed.ends_with(",}") {
                report.add_info(
                    "Trailing comma before closing brace (allowed in modern TS)".to_string(),
                    Some(format!("{}:{}", file_name, line_num + 1)),
                );
            }

            // Missing semicolons (warning, not error in TS)
            if self.should_have_semicolon(trimmed)
                && !trimmed.ends_with(';')
                && !trimmed.ends_with('{')
            {
                report.add_info(
                    "Statement may benefit from explicit semicolon".to_string(),
                    Some(format!("{}:{}", file_name, line_num + 1)),
                );
            }

            // Assignment in conditional
            if (trimmed.contains("if (") || trimmed.contains("while ("))
                && trimmed.contains(" = ")
                && !trimmed.contains("==")
            {
                if !trimmed.contains("===") {
                    report.add_warning(
                        "Possible assignment in conditional (use === for comparison)".to_string(),
                        Some(format!("{}:{}", file_name, line_num + 1)),
                        Some(
                            "Use '===' for comparison or wrap assignment in parentheses"
                                .to_string(),
                        ),
                    );
                }
            }

            // Using 'any' type
            if trimmed.contains(": any") || trimmed.contains("<any>") {
                report.add_info(
                    "Using 'any' type reduces type safety".to_string(),
                    Some(format!("{}:{}", file_name, line_num + 1)),
                );
            }
        }
    }

    /// Validate type declarations
    fn validate_type_declarations(
        &self,
        code: &str,
        report: &mut ValidationReport,
        file_name: &str,
    ) {
        let interface_regex =
            Regex::new(r"^\s*interface\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\{?").unwrap();
        let type_regex = Regex::new(r"^\s*type\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*=").unwrap();

        for (line_num, line) in code.lines().enumerate() {
            // Check interface declarations
            if let Some(caps) = interface_regex.captures(line) {
                let name = &caps[1];

                // Check PascalCase convention
                if !name.chars().next().unwrap().is_uppercase() {
                    report.add_warning(
                        format!("Interface '{}' should use PascalCase", name),
                        Some(format!("{}:{}", file_name, line_num + 1)),
                        Some("Use PascalCase for type names (e.g., MyInterface)".to_string()),
                    );
                }
            }

            // Check type alias declarations
            if let Some(caps) = type_regex.captures(line) {
                let name = &caps[1];

                // Check PascalCase convention
                if !name.chars().next().unwrap().is_uppercase() {
                    report.add_warning(
                        format!("Type alias '{}' should use PascalCase", name),
                        Some(format!("{}:{}", file_name, line_num + 1)),
                        Some("Use PascalCase for type names (e.g., MyType)".to_string()),
                    );
                }
            }
        }
    }

    /// Validate function declarations
    fn validate_function_declarations(
        &self,
        code: &str,
        report: &mut ValidationReport,
        file_name: &str,
    ) {
        let function_regex = Regex::new(
            r"^\s*(?:export\s+)?(?:async\s+)?function\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(",
        )
        .unwrap();
        let arrow_function_regex = Regex::new(r"^\s*(?:export\s+)?(?:const|let|var)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*=\s*(?:async\s+)?\(").unwrap();

        for (line_num, line) in code.lines().enumerate() {
            // Check function declarations
            if let Some(caps) = function_regex.captures(line) {
                let name = &caps[1];

                // Check camelCase convention
                if name.chars().next().unwrap().is_uppercase() {
                    report.add_warning(
                        format!("Function '{}' should use camelCase", name),
                        Some(format!("{}:{}", file_name, line_num + 1)),
                        Some("Use camelCase for function names (e.g., myFunction)".to_string()),
                    );
                }
            }

            // Check arrow functions
            if arrow_function_regex.is_match(line) {
                // Arrow function validation
                if !line.contains("=>") {
                    let next_line = code.lines().nth(line_num + 1).unwrap_or("");
                    if !next_line.contains("=>") {
                        report.add_warning(
                            "Arrow function may be incomplete".to_string(),
                            Some(format!("{}:{}", file_name, line_num + 1)),
                            Some(
                                "Check arrow function syntax: const fn = (params) => body"
                                    .to_string(),
                            ),
                        );
                    }
                }
            }
        }
    }

    /// Remove strings and comments from line (simplified)
    fn remove_strings_and_comments(&self, line: &str) -> String {
        let mut result = String::new();
        let mut in_string = false;
        let mut in_comment = false;
        let mut escape_next = false;
        let mut prev_char = ' ';

        for ch in line.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' | '\'' | '`' => in_string = !in_string,
                '/' if !in_string && prev_char == '/' => {
                    in_comment = true;
                    result.pop(); // Remove first '/'
                    break;
                }
                _ if !in_string && !in_comment => result.push(ch),
                _ => {}
            }
            prev_char = ch;
        }

        result
    }

    /// Check if line should have semicolon
    fn should_have_semicolon(&self, line: &str) -> bool {
        let trimmed = line.trim();

        // Lines that typically need semicolons
        (trimmed.starts_with("const ")
            || trimmed.starts_with("let ")
            || trimmed.starts_with("var ")
            || trimmed.starts_with("return ")
            || trimmed.starts_with("throw "))
            && !trimmed.ends_with('{')
            && !trimmed.ends_with('(')
    }

    /// Reset state for new validation
    pub fn reset(&mut self) {
        self.seen_identifiers.clear();
    }
}

impl Default for TypeScriptValidator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// YAML Validator
// =============================================================================

/// Validates YAML syntax using serde_yaml
pub struct YamlValidator;

impl YamlValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validate YAML syntax
    pub fn validate(&self, content: &str, file_name: &str) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();

        match serde_yaml::from_str::<YamlValue>(content) {
            Ok(_) => {
                report.add_info(
                    format!("YAML syntax validation passed for {}", file_name),
                    None,
                );
            }
            Err(e) => {
                let location = e
                    .location()
                    .map(|loc| format!("{}:{}:{}", file_name, loc.line(), loc.column()));

                report.add_error(
                    format!("YAML syntax error: {}", e),
                    location,
                    Some(self.suggest_yaml_fix(&e.to_string())),
                );
            }
        }

        // Additional YAML-specific checks
        self.validate_yaml_structure(content, &mut report, file_name);

        Ok(report)
    }

    /// Validate YAML structure and common issues
    fn validate_yaml_structure(
        &self,
        content: &str,
        report: &mut ValidationReport,
        file_name: &str,
    ) {
        for (line_num, line) in content.lines().enumerate() {
            // Check for tabs (YAML requires spaces)
            if line.contains('\t') {
                report.add_error(
                    "YAML does not allow tabs for indentation".to_string(),
                    Some(format!("{}:{}", file_name, line_num + 1)),
                    Some("Replace tabs with spaces".to_string()),
                );
            }

            // Check for inconsistent indentation
            if line.starts_with(' ') {
                let spaces = line.chars().take_while(|c| *c == ' ').count();
                if spaces % 2 != 0 {
                    report.add_warning(
                        "Inconsistent indentation (should be multiples of 2)".to_string(),
                        Some(format!("{}:{}", file_name, line_num + 1)),
                        Some("Use consistent 2-space indentation".to_string()),
                    );
                }
            }

            // Check for trailing spaces
            if line.ends_with(' ') && !line.trim().is_empty() {
                report.add_info(
                    "Line has trailing whitespace".to_string(),
                    Some(format!("{}:{}", file_name, line_num + 1)),
                );
            }
        }
    }

    /// Suggest fix for common YAML errors
    fn suggest_yaml_fix(&self, error_msg: &str) -> String {
        if error_msg.contains("tab") {
            "Replace tabs with spaces for indentation".to_string()
        } else if error_msg.contains("indent") {
            "Check YAML indentation (use 2 spaces per level)".to_string()
        } else if error_msg.contains("key") {
            "Check for duplicate keys or invalid key format".to_string()
        } else if error_msg.contains("colon") {
            "Ensure key-value pairs have space after colon (key: value)".to_string()
        } else {
            "Check YAML syntax documentation".to_string()
        }
    }
}

impl Default for YamlValidator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// JSON Validator
// =============================================================================

/// Validates JSON syntax using serde_json
pub struct JsonValidator;

impl JsonValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validate JSON syntax
    pub fn validate(&self, content: &str, file_name: &str) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();

        match serde_json::from_str::<JsonValue>(content) {
            Ok(_) => {
                report.add_info(
                    format!("JSON syntax validation passed for {}", file_name),
                    None,
                );
            }
            Err(e) => {
                let location = Some(format!("{}:{}:{}", file_name, e.line(), e.column()));

                report.add_error(
                    format!("JSON syntax error: {}", e),
                    location,
                    Some(self.suggest_json_fix(&e.to_string())),
                );
            }
        }

        Ok(report)
    }

    /// Suggest fix for common JSON errors
    fn suggest_json_fix(&self, error_msg: &str) -> String {
        if error_msg.contains("trailing comma") {
            "Remove trailing comma before closing bracket/brace".to_string()
        } else if error_msg.contains("expected") && error_msg.contains("at") {
            "Check for missing or extra commas, brackets, or braces".to_string()
        } else if error_msg.contains("key") {
            "JSON keys must be strings in double quotes".to_string()
        } else if error_msg.contains("EOF") {
            "JSON document is incomplete (check for unclosed structures)".to_string()
        } else {
            "Check JSON syntax (keys in quotes, no trailing commas)".to_string()
        }
    }
}

impl Default for JsonValidator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// OpenAPI Validator
// =============================================================================

/// Validates OpenAPI specifications (YAML + OpenAPI schema)
pub struct OpenApiValidator {
    yaml_validator: YamlValidator,
}

impl OpenApiValidator {
    pub fn new() -> Self {
        Self {
            yaml_validator: YamlValidator::new(),
        }
    }

    /// Validate OpenAPI specification
    pub fn validate(&self, content: &str, file_name: &str) -> Result<ValidationReport> {
        let mut report = self.yaml_validator.validate(content, file_name)?;

        // If YAML is invalid, don't proceed with OpenAPI validation
        if report.has_errors() {
            return Ok(report);
        }

        // Parse YAML and validate OpenAPI structure
        match serde_yaml::from_str::<YamlValue>(content) {
            Ok(yaml) => {
                self.validate_openapi_structure(&yaml, &mut report, file_name);
            }
            Err(_) => {
                // Error already reported by YAML validator
            }
        }

        Ok(report)
    }

    /// Validate OpenAPI structure
    fn validate_openapi_structure(
        &self,
        yaml: &YamlValue,
        report: &mut ValidationReport,
        file_name: &str,
    ) {
        // Check for required top-level fields
        if let YamlValue::Mapping(map) = yaml {
            // Check for openapi version
            if !map.contains_key(&YamlValue::String("openapi".to_string())) {
                report.add_error(
                    "Missing required field 'openapi'".to_string(),
                    Some(file_name.to_string()),
                    Some("Add 'openapi: 3.0.0' or later version".to_string()),
                );
            }

            // Check for info section
            if !map.contains_key(&YamlValue::String("info".to_string())) {
                report.add_error(
                    "Missing required field 'info'".to_string(),
                    Some(file_name.to_string()),
                    Some("Add 'info' section with title and version".to_string()),
                );
            } else {
                // Validate info section
                if let Some(YamlValue::Mapping(info)) =
                    map.get(&YamlValue::String("info".to_string()))
                {
                    if !info.contains_key(&YamlValue::String("title".to_string())) {
                        report.add_error(
                            "Missing required field 'info.title'".to_string(),
                            Some(file_name.to_string()),
                            Some("Add 'title' field in info section".to_string()),
                        );
                    }
                    if !info.contains_key(&YamlValue::String("version".to_string())) {
                        report.add_error(
                            "Missing required field 'info.version'".to_string(),
                            Some(file_name.to_string()),
                            Some("Add 'version' field in info section".to_string()),
                        );
                    }
                }
            }

            // Check for paths section
            if !map.contains_key(&YamlValue::String("paths".to_string())) {
                report.add_warning(
                    "Missing 'paths' section".to_string(),
                    Some(file_name.to_string()),
                    Some("OpenAPI specs typically include a 'paths' section".to_string()),
                );
            }

            // Validate OpenAPI version format
            if let Some(YamlValue::String(version)) =
                map.get(&YamlValue::String("openapi".to_string()))
            {
                if !version.starts_with("3.") {
                    report.add_warning(
                        format!("OpenAPI version '{}' may not be supported", version),
                        Some(file_name.to_string()),
                        Some("Consider using OpenAPI 3.0.0 or later".to_string()),
                    );
                }
            }
        } else {
            report.add_error(
                "OpenAPI specification must be a YAML mapping".to_string(),
                Some(file_name.to_string()),
                Some("Start with top-level fields: openapi, info, paths".to_string()),
            );
        }
    }
}

impl Default for OpenApiValidator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get TypeScript reserved words
fn get_typescript_reserved_words() -> HashSet<&'static str> {
    [
        "abstract",
        "any",
        "as",
        "async",
        "await",
        "boolean",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "debugger",
        "declare",
        "default",
        "delete",
        "do",
        "else",
        "enum",
        "export",
        "extends",
        "false",
        "finally",
        "for",
        "from",
        "function",
        "get",
        "if",
        "implements",
        "import",
        "in",
        "infer",
        "instanceof",
        "interface",
        "is",
        "keyof",
        "let",
        "module",
        "namespace",
        "never",
        "new",
        "null",
        "number",
        "object",
        "of",
        "package",
        "private",
        "protected",
        "public",
        "readonly",
        "require",
        "return",
        "set",
        "static",
        "string",
        "super",
        "switch",
        "symbol",
        "this",
        "throw",
        "true",
        "try",
        "type",
        "typeof",
        "undefined",
        "unique",
        "unknown",
        "var",
        "void",
        "while",
        "with",
        "yield",
    ]
    .iter()
    .copied()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typescript_balanced_braces() {
        let mut validator = TypeScriptValidator::new();

        let valid_code = "function test() { return { key: 'value' }; }";
        let report = validator.validate(valid_code, "test.ts").unwrap();
        assert!(!report.has_errors());

        validator.reset();
        let invalid_code = "function test() { return { key: 'value' };";
        let report = validator.validate(invalid_code, "test.ts").unwrap();
        assert!(report.has_errors());
    }

    #[test]
    fn test_typescript_import_syntax() {
        let mut validator = TypeScriptValidator::new();

        let valid_import = "import { Component } from 'react';";
        let report = validator.validate(valid_import, "test.ts").unwrap();
        assert!(!report.has_errors());

        validator.reset();
        let invalid_import = "import { Component } form 'react';";
        let report = validator.validate(invalid_import, "test.ts").unwrap();
        assert!(report.has_errors());
    }

    #[test]
    fn test_yaml_validator() {
        let validator = YamlValidator::new();

        let valid_yaml = "key: value\nlist:\n  - item1\n  - item2";
        let report = validator.validate(valid_yaml, "test.yaml").unwrap();
        assert!(!report.has_errors());

        let invalid_yaml = "key: value\n\tindented: with_tab";
        let report = validator.validate(invalid_yaml, "test.yaml").unwrap();
        assert!(report.has_errors());
    }

    #[test]
    fn test_json_validator() {
        let validator = JsonValidator::new();

        let valid_json = r#"{"key": "value", "array": [1, 2, 3]}"#;
        let report = validator.validate(valid_json, "test.json").unwrap();
        assert!(!report.has_errors());

        let invalid_json = r#"{"key": "value",}"#; // Trailing comma
        let report = validator.validate(invalid_json, "test.json").unwrap();
        assert!(report.has_errors());
    }

    #[test]
    fn test_openapi_validator() {
        let validator = OpenApiValidator::new();

        let valid_openapi = r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /test:
    get:
      summary: Test endpoint
"#;
        let report = validator.validate(valid_openapi, "openapi.yaml").unwrap();
        assert!(!report.has_errors());

        let invalid_openapi = r#"
info:
  title: Test API
"#; // Missing openapi version
        let report = validator.validate(invalid_openapi, "openapi.yaml").unwrap();
        assert!(report.has_errors());
    }

    #[test]
    fn test_typescript_reserved_words() {
        let mut validator = TypeScriptValidator::new();

        let invalid_code = "const class = 'test';";
        let report = validator.validate(invalid_code, "test.ts").unwrap();
        assert!(report.has_errors());
    }

    #[test]
    fn test_typescript_type_declarations() {
        let mut validator = TypeScriptValidator::new();

        let code = "interface myInterface { key: string; }";
        let report = validator.validate(code, "test.ts").unwrap();
        assert!(report.warning_count > 0); // Should warn about lowercase interface name
    }
}
