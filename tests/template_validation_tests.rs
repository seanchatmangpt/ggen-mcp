//! Template Parameter Validation Tests
//!
//! Comprehensive test suite for template parameter validation,
//! covering both valid and invalid parameter scenarios.

use serde_json::json;
use spreadsheet_mcp::template::{
    ParameterDefinition, ParameterSchema, ParameterType, TemplateContext, ValidationError,
    ValidationRule,
};
use std::collections::HashMap;

// ============================================================================
// PARAMETER TYPE TESTS
// ============================================================================

#[test]
fn test_parameter_type_string_matches() {
    let param_type = ParameterType::String;
    assert!(param_type.matches(&json!("hello")));
    assert!(!param_type.matches(&json!(42)));
    assert!(!param_type.matches(&json!(true)));
    assert!(!param_type.matches(&json!(null)));
}

#[test]
fn test_parameter_type_bool_matches() {
    let param_type = ParameterType::Bool;
    assert!(param_type.matches(&json!(true)));
    assert!(param_type.matches(&json!(false)));
    assert!(!param_type.matches(&json!("true")));
    assert!(!param_type.matches(&json!(1)));
}

#[test]
fn test_parameter_type_number_matches() {
    let param_type = ParameterType::Number;
    assert!(param_type.matches(&json!(42)));
    assert!(param_type.matches(&json!(-10)));
    assert!(!param_type.matches(&json!(42.5))); // Float, not integer
    assert!(!param_type.matches(&json!("42")));
}

#[test]
fn test_parameter_type_float_matches() {
    let param_type = ParameterType::Float;
    assert!(param_type.matches(&json!(42.5)));
    assert!(param_type.matches(&json!(42))); // Integers are valid floats
    assert!(!param_type.matches(&json!("42.5")));
}

#[test]
fn test_parameter_type_array_matches() {
    let param_type = ParameterType::Array(Box::new(ParameterType::String));
    assert!(param_type.matches(&json!(["a", "b", "c"])));
    assert!(!param_type.matches(&json!([1, 2, 3])));
    assert!(!param_type.matches(&json!("not an array")));

    // Empty array should match
    assert!(param_type.matches(&json!([])));

    // Mixed types should not match
    assert!(!param_type.matches(&json!(["a", 1, "c"])));
}

#[test]
fn test_parameter_type_optional_matches() {
    let param_type = ParameterType::Optional(Box::new(ParameterType::String));
    assert!(param_type.matches(&json!("hello")));
    assert!(param_type.matches(&json!(null)));
    assert!(!param_type.matches(&json!(42)));
}

#[test]
fn test_parameter_type_any_matches() {
    let param_type = ParameterType::Any;
    assert!(param_type.matches(&json!("string")));
    assert!(param_type.matches(&json!(42)));
    assert!(param_type.matches(&json!(true)));
    assert!(param_type.matches(&json!(null)));
    assert!(param_type.matches(&json!(["array"])));
    assert!(param_type.matches(&json!({"key": "value"})));
}

// ============================================================================
// VALIDATION RULE TESTS
// ============================================================================

#[test]
fn test_validation_rule_min_length_string() {
    let rule = ValidationRule::MinLength(3);

    // Too short
    assert!(rule.validate("field", &json!("ab")).is_err());

    // Exact length
    assert!(rule.validate("field", &json!("abc")).is_ok());

    // Longer
    assert!(rule.validate("field", &json!("abcd")).is_ok());
}

#[test]
fn test_validation_rule_min_length_array() {
    let rule = ValidationRule::MinLength(2);

    // Too short
    assert!(rule.validate("field", &json!([1])).is_err());

    // Exact length
    assert!(rule.validate("field", &json!([1, 2])).is_ok());

    // Longer
    assert!(rule.validate("field", &json!([1, 2, 3])).is_ok());
}

#[test]
fn test_validation_rule_max_length_string() {
    let rule = ValidationRule::MaxLength(5);

    // Under limit
    assert!(rule.validate("field", &json!("abc")).is_ok());

    // At limit
    assert!(rule.validate("field", &json!("abcde")).is_ok());

    // Over limit
    assert!(rule.validate("field", &json!("abcdef")).is_err());
}

#[test]
fn test_validation_rule_min_number() {
    let rule = ValidationRule::Min(10);

    assert!(rule.validate("field", &json!(5)).is_err());
    assert!(rule.validate("field", &json!(10)).is_ok());
    assert!(rule.validate("field", &json!(15)).is_ok());
}

#[test]
fn test_validation_rule_max_number() {
    let rule = ValidationRule::Max(100);

    assert!(rule.validate("field", &json!(50)).is_ok());
    assert!(rule.validate("field", &json!(100)).is_ok());
    assert!(rule.validate("field", &json!(150)).is_err());
}

#[test]
fn test_validation_rule_regex() {
    let rule = ValidationRule::Regex(regex::Regex::new(r"^[a-z]+$").unwrap());

    assert!(rule.validate("field", &json!("hello")).is_ok());
    assert!(rule.validate("field", &json!("Hello")).is_err()); // Capital letter
    assert!(rule.validate("field", &json!("hello123")).is_err()); // Contains digits
}

#[test]
fn test_validation_rule_not_empty_string() {
    let rule = ValidationRule::NotEmpty;

    assert!(rule.validate("field", &json!("")).is_err());
    assert!(rule.validate("field", &json!("a")).is_ok());
}

#[test]
fn test_validation_rule_not_empty_array() {
    let rule = ValidationRule::NotEmpty;

    assert!(rule.validate("field", &json!([])).is_err());
    assert!(rule.validate("field", &json!([1])).is_ok());
}

#[test]
fn test_validation_rule_one_of() {
    let rule = ValidationRule::OneOf(vec![json!("debug"), json!("release"), json!("test")]);

    assert!(rule.validate("field", &json!("debug")).is_ok());
    assert!(rule.validate("field", &json!("release")).is_ok());
    assert!(rule.validate("field", &json!("production")).is_err());
}

// ============================================================================
// PARAMETER DEFINITION TESTS
// ============================================================================

#[test]
fn test_parameter_definition_required() {
    let param = ParameterDefinition::new("name", ParameterType::String).required();

    assert!(param.required);
    assert_eq!(param.name, "name");
}

#[test]
fn test_parameter_definition_default() {
    let param = ParameterDefinition::new("count", ParameterType::Number).default(json!(10));

    assert!(!param.required);
    assert_eq!(param.default, Some(json!(10)));
}

#[test]
fn test_parameter_definition_validate_success() {
    let param = ParameterDefinition::new("name", ParameterType::String)
        .required()
        .rule(ValidationRule::MinLength(3));

    assert!(param.validate(&json!("hello")).is_ok());
}

#[test]
fn test_parameter_definition_validate_type_mismatch() {
    let param = ParameterDefinition::new("name", ParameterType::String);

    let result = param.validate(&json!(42));
    assert!(result.is_err());

    if let Err(ValidationError::TypeMismatch {
        name,
        expected,
        actual,
    }) = result
    {
        assert_eq!(name, "name");
        assert_eq!(expected, "String");
        assert_eq!(actual, "Number");
    } else {
        panic!("Expected TypeMismatch error");
    }
}

#[test]
fn test_parameter_definition_validate_rule_failure() {
    let param =
        ParameterDefinition::new("name", ParameterType::String).rule(ValidationRule::MinLength(5));

    let result = param.validate(&json!("abc"));
    assert!(result.is_err());
}

// ============================================================================
// PARAMETER SCHEMA TESTS
// ============================================================================

#[test]
fn test_parameter_schema_basic() {
    let schema = ParameterSchema::new("test.tera")
        .parameter(ParameterDefinition::new("name", ParameterType::String).required());

    assert_eq!(schema.template_name, "test.tera");
    assert!(schema.get_parameter("name").is_some());
    assert!(schema.get_parameter("missing").is_none());
}

#[test]
fn test_parameter_schema_validate_success() {
    let schema = ParameterSchema::new("test.tera")
        .parameter(ParameterDefinition::new("name", ParameterType::String).required())
        .parameter(ParameterDefinition::new("count", ParameterType::Number).default(json!(0)));

    let mut context = HashMap::new();
    context.insert("name".to_string(), json!("test"));

    assert!(schema.validate_context(&context).is_ok());
}

#[test]
fn test_parameter_schema_validate_missing_required() {
    let schema = ParameterSchema::new("test.tera")
        .parameter(ParameterDefinition::new("name", ParameterType::String).required());

    let context = HashMap::new();

    let result = schema.validate_context(&context);
    assert!(result.is_err());

    if let Err(errors) = result {
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::MissingRequired(_)));
    }
}

#[test]
fn test_parameter_schema_validate_unknown_parameter() {
    let schema = ParameterSchema::new("test.tera");

    let mut context = HashMap::new();
    context.insert("unknown_param".to_string(), json!("value"));

    let result = schema.validate_context(&context);
    assert!(result.is_err());

    if let Err(errors) = result {
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::UnknownParameter(_)));
    }
}

#[test]
fn test_parameter_schema_allow_unknown() {
    let schema = ParameterSchema::new("test.tera").allow_unknown();

    let mut context = HashMap::new();
    context.insert("unknown_param".to_string(), json!("value"));

    assert!(schema.validate_context(&context).is_ok());
}

#[test]
fn test_parameter_schema_multiple_errors() {
    let schema = ParameterSchema::new("test.tera")
        .parameter(ParameterDefinition::new("name", ParameterType::String).required())
        .parameter(ParameterDefinition::new("count", ParameterType::Number).required());

    let context = HashMap::new();

    let result = schema.validate_context(&context);
    assert!(result.is_err());

    if let Err(errors) = result {
        assert_eq!(errors.len(), 2);
    }
}

// ============================================================================
// TEMPLATE CONTEXT TESTS
// ============================================================================

#[test]
fn test_template_context_insert_string() {
    let mut ctx = TemplateContext::new("test.tera");
    assert!(ctx.insert_string("name", "value").is_ok());

    assert_eq!(ctx.get("name"), Some(&json!("value")));
}

#[test]
fn test_template_context_insert_bool() {
    let mut ctx = TemplateContext::new("test.tera");
    assert!(ctx.insert_bool("flag", true).is_ok());

    assert_eq!(ctx.get("flag"), Some(&json!(true)));
}

#[test]
fn test_template_context_insert_number() {
    let mut ctx = TemplateContext::new("test.tera");
    assert!(ctx.insert_number("count", 42).is_ok());

    assert_eq!(ctx.get("count"), Some(&json!(42)));
}

#[test]
fn test_template_context_insert_array() {
    let mut ctx = TemplateContext::new("test.tera");
    let arr = vec![json!("a"), json!("b")];
    assert!(ctx.insert_array("items", arr).is_ok());

    assert_eq!(ctx.get("items"), Some(&json!(["a", "b"])));
}

#[test]
fn test_template_context_contains() {
    let mut ctx = TemplateContext::new("test.tera");
    ctx.insert_string("name", "value").unwrap();

    assert!(ctx.contains("name"));
    assert!(!ctx.contains("missing"));
}

#[test]
fn test_template_context_remove() {
    let mut ctx = TemplateContext::new("test.tera");
    ctx.insert_string("name", "value").unwrap();

    assert!(ctx.contains("name"));
    assert_eq!(ctx.remove("name"), Some(json!("value")));
    assert!(!ctx.contains("name"));
}

#[test]
fn test_template_context_unused_parameters() {
    let mut ctx = TemplateContext::new("test.tera");
    ctx.insert_string("used", "value1").unwrap();
    ctx.insert_string("unused1", "value2").unwrap();
    ctx.insert_string("unused2", "value3").unwrap();

    ctx.mark_used("used");

    let unused = ctx.unused_parameters();
    assert_eq!(unused.len(), 2);
    assert!(unused.contains(&"unused1".to_string()));
    assert!(unused.contains(&"unused2".to_string()));
}

#[test]
fn test_template_context_template_name() {
    let ctx = TemplateContext::new("domain_entity.rs.tera");
    assert_eq!(ctx.template_name(), "domain_entity.rs.tera");
}

#[test]
fn test_template_context_parameter_names() {
    let mut ctx = TemplateContext::new("test.tera");
    ctx.insert_string("name", "value").unwrap();
    ctx.insert_bool("flag", true).unwrap();

    let names = ctx.parameter_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"name"));
    assert!(names.contains(&"flag"));
}

// ============================================================================
// INVALID PARAMETER TESTS
// ============================================================================

#[test]
fn test_invalid_entity_name_empty() {
    let schema = ParameterSchema::new("domain_entity.rs.tera").parameter(
        ParameterDefinition::new("entity_name", ParameterType::String)
            .required()
            .rule(ValidationRule::NotEmpty),
    );

    let mut context = HashMap::new();
    context.insert("entity_name".to_string(), json!(""));

    let result = schema.validate_context(&context);
    assert!(result.is_err());
}

#[test]
fn test_invalid_entity_name_special_chars() {
    let schema = ParameterSchema::new("domain_entity.rs.tera").parameter(
        ParameterDefinition::new("entity_name", ParameterType::String)
            .required()
            .rule(ValidationRule::Regex(
                regex::Regex::new(r"^[A-Za-z][A-Za-z0-9_]*$").unwrap(),
            )),
    );

    let mut context = HashMap::new();
    context.insert("entity_name".to_string(), json!("Invalid-Name!"));

    let result = schema.validate_context(&context);
    assert!(result.is_err());
}

#[test]
fn test_invalid_number_as_string() {
    let schema = ParameterSchema::new("test.tera")
        .parameter(ParameterDefinition::new("count", ParameterType::Number).required());

    let mut context = HashMap::new();
    context.insert("count".to_string(), json!("42")); // String instead of number

    let result = schema.validate_context(&context);
    assert!(result.is_err());

    if let Err(errors) = result {
        assert!(matches!(errors[0], ValidationError::TypeMismatch { .. }));
    }
}

#[test]
fn test_invalid_bool_as_string() {
    let schema = ParameterSchema::new("test.tera")
        .parameter(ParameterDefinition::new("enabled", ParameterType::Bool).required());

    let mut context = HashMap::new();
    context.insert("enabled".to_string(), json!("true")); // String instead of bool

    let result = schema.validate_context(&context);
    assert!(result.is_err());
}

#[test]
fn test_invalid_array_element_type() {
    let schema = ParameterSchema::new("test.tera").parameter(
        ParameterDefinition::new(
            "items",
            ParameterType::Array(Box::new(ParameterType::String)),
        )
        .required(),
    );

    let mut context = HashMap::new();
    context.insert("items".to_string(), json!([1, 2, 3])); // Numbers instead of strings

    let result = schema.validate_context(&context);
    assert!(result.is_err());
}

#[test]
fn test_invalid_missing_required_field() {
    let schema = ParameterSchema::new("test.tera")
        .parameter(ParameterDefinition::new("required1", ParameterType::String).required())
        .parameter(ParameterDefinition::new("required2", ParameterType::String).required())
        .parameter(ParameterDefinition::new("required3", ParameterType::String).required());

    let mut context = HashMap::new();
    context.insert("required1".to_string(), json!("value1"));
    // Missing required2 and required3

    let result = schema.validate_context(&context);
    assert!(result.is_err());

    if let Err(errors) = result {
        assert_eq!(errors.len(), 2);
    }
}

#[test]
fn test_invalid_value_too_long() {
    let schema = ParameterSchema::new("test.tera").parameter(
        ParameterDefinition::new("short_string", ParameterType::String)
            .required()
            .rule(ValidationRule::MaxLength(10)),
    );

    let mut context = HashMap::new();
    context.insert(
        "short_string".to_string(),
        json!("this string is way too long"),
    );

    let result = schema.validate_context(&context);
    assert!(result.is_err());
}

#[test]
fn test_invalid_number_out_of_range() {
    let schema = ParameterSchema::new("test.tera").parameter(
        ParameterDefinition::new("percentage", ParameterType::Number)
            .required()
            .rule(ValidationRule::Min(0))
            .rule(ValidationRule::Max(100)),
    );

    let mut context = HashMap::new();
    context.insert("percentage".to_string(), json!(150));

    let result = schema.validate_context(&context);
    assert!(result.is_err());
}

#[test]
fn test_invalid_typo_in_parameter_name() {
    let schema = ParameterSchema::new("test.tera")
        .parameter(ParameterDefinition::new("entity_name", ParameterType::String).required());

    let mut context = HashMap::new();
    context.insert("entitiy_name".to_string(), json!("User")); // Typo: entitiy vs entity

    let result = schema.validate_context(&context);
    assert!(result.is_err());

    // Should have 2 errors: missing required and unknown parameter
    if let Err(errors) = result {
        assert_eq!(errors.len(), 2);
    }
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_empty_array_validation() {
    let schema = ParameterSchema::new("test.tera").parameter(
        ParameterDefinition::new(
            "items",
            ParameterType::Array(Box::new(ParameterType::String)),
        )
        .default(json!([])),
    );

    let mut context = HashMap::new();
    context.insert("items".to_string(), json!([]));

    assert!(schema.validate_context(&context).is_ok());
}

#[test]
fn test_null_for_optional_parameter() {
    let schema = ParameterSchema::new("test.tera").parameter(
        ParameterDefinition::new(
            "optional_field",
            ParameterType::Optional(Box::new(ParameterType::String)),
        )
        .default(json!(null)),
    );

    let mut context = HashMap::new();
    context.insert("optional_field".to_string(), json!(null));

    assert!(schema.validate_context(&context).is_ok());
}

#[test]
fn test_very_long_string() {
    let schema = ParameterSchema::new("test.tera").parameter(
        ParameterDefinition::new("text", ParameterType::String)
            .required()
            .rule(ValidationRule::MaxLength(1000)),
    );

    let long_string = "a".repeat(2000);
    let mut context = HashMap::new();
    context.insert("text".to_string(), json!(long_string));

    let result = schema.validate_context(&context);
    assert!(result.is_err());
}

#[test]
fn test_nested_array() {
    let inner_type = ParameterType::Array(Box::new(ParameterType::String));
    let outer_type = ParameterType::Array(Box::new(inner_type));

    let schema = ParameterSchema::new("test.tera")
        .parameter(ParameterDefinition::new("nested", outer_type).required());

    let mut context = HashMap::new();
    context.insert("nested".to_string(), json!([["a", "b"], ["c", "d"]]));

    assert!(schema.validate_context(&context).is_ok());
}

#[test]
fn test_multiple_validation_rules() {
    let schema = ParameterSchema::new("test.tera").parameter(
        ParameterDefinition::new("username", ParameterType::String)
            .required()
            .rules(vec![
                ValidationRule::NotEmpty,
                ValidationRule::MinLength(3),
                ValidationRule::MaxLength(20),
                ValidationRule::Regex(regex::Regex::new(r"^[a-zA-Z0-9_]+$").unwrap()),
            ]),
    );

    // Valid username
    let mut valid_context = HashMap::new();
    valid_context.insert("username".to_string(), json!("valid_user123"));
    assert!(schema.validate_context(&valid_context).is_ok());

    // Too short
    let mut short_context = HashMap::new();
    short_context.insert("username".to_string(), json!("ab"));
    assert!(schema.validate_context(&short_context).is_err());

    // Invalid characters
    let mut invalid_context = HashMap::new();
    invalid_context.insert("username".to_string(), json!("invalid-user!"));
    assert!(schema.validate_context(&invalid_context).is_err());
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[test]
fn test_domain_entity_template_valid_context() {
    use spreadsheet_mcp::template::schemas::get_schema;

    if let Some(schema) = get_schema("domain_entity.rs.tera") {
        let mut context = HashMap::new();
        context.insert("entity_name".to_string(), json!("User"));
        context.insert("description".to_string(), json!("User entity"));
        context.insert("has_id".to_string(), json!(true));
        context.insert("has_timestamps".to_string(), json!(false));
        context.insert("has_validation".to_string(), json!(true));
        context.insert("has_builder".to_string(), json!(true));
        context.insert("fields".to_string(), json!([]));
        context.insert("invariants".to_string(), json!([]));

        let result = schema.validate_context(&context);
        if let Err(ref errors) = result {
            eprintln!("Validation errors: {:?}", errors);
        }
        assert!(result.is_ok());
    }
}

#[test]
fn test_mcp_tool_handler_template_valid_context() {
    use spreadsheet_mcp::template::schemas::get_schema;

    if let Some(schema) = get_schema("mcp_tool_handler.rs.tera") {
        let mut context = HashMap::new();
        context.insert("tool_name".to_string(), json!("list_items"));
        context.insert("description".to_string(), json!("Lists all items"));
        context.insert("category".to_string(), json!("query"));
        context.insert("has_params".to_string(), json!(true));
        context.insert("has_pagination".to_string(), json!(false));
        context.insert("has_filters".to_string(), json!(false));
        context.insert("params".to_string(), json!([]));
        context.insert("response_fields".to_string(), json!([]));

        let result = schema.validate_context(&context);
        if let Err(ref errors) = result {
            eprintln!("Validation errors: {:?}", errors);
        }
        assert!(result.is_ok());
    }
}

#[test]
fn test_template_context_builder_pattern() {
    let mut ctx = TemplateContext::new("test.tera");

    // Builder pattern usage
    ctx.insert_string("name", "Test")
        .unwrap()
        .insert_bool("enabled", true)
        .unwrap()
        .insert_number("count", 42)
        .unwrap();

    assert_eq!(ctx.parameter_names().len(), 3);
    assert!(ctx.contains("name"));
    assert!(ctx.contains("enabled"));
    assert!(ctx.contains("count"));
}
