use super::parse_dsl;
use super::types::{Operator, Value};

#[test]
fn test_simple_exact_match() {
    let result = parse_dsl("type:gpu").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].field, "type");
    assert_eq!(result[0].operator, Operator::Eq);
    assert_eq!(result[0].values, vec![Value::String("gpu".to_string())]);
    assert!(!result[0].negated);
}

#[test]
fn test_gte_operator() {
    let result = parse_dsl("price:>=100").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].field, "price");
    assert_eq!(result[0].operator, Operator::Gte);
    assert_eq!(result[0].values, vec![Value::Integer(100)]);
}

#[test]
fn test_lte_operator() {
    let result = parse_dsl("price:<=500").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].field, "price");
    assert_eq!(result[0].operator, Operator::Lte);
    assert_eq!(result[0].values, vec![Value::Integer(500)]);
}

#[test]
fn test_gt_operator() {
    let result = parse_dsl("cores:>8").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].operator, Operator::Gt);
}

#[test]
fn test_lt_operator() {
    let result = parse_dsl("cores:<16").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].operator, Operator::Lt);
}

#[test]
fn test_range() {
    let result = parse_dsl("price:[50 TO 200]").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].field, "price");
    assert_eq!(result[0].operator, Operator::Range);
    assert_eq!(
        result[0].values,
        vec![Value::Integer(50), Value::Integer(200)]
    );
}

#[test]
fn test_or_group() {
    let result = parse_dsl("type:(gpu OR compute)").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].field, "type");
    assert_eq!(result[0].operator, Operator::Eq);
    assert_eq!(
        result[0].values,
        vec![
            Value::String("gpu".to_string()),
            Value::String("compute".to_string())
        ]
    );
}

#[test]
fn test_or_group_three_values() {
    let result = parse_dsl("country:(US OR DE OR NL)").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].values.len(), 3);
    assert_eq!(
        result[0].values,
        vec![
            Value::String("US".to_string()),
            Value::String("DE".to_string()),
            Value::String("NL".to_string())
        ]
    );
}

#[test]
fn test_negation_exclamation() {
    let result = parse_dsl("!stock:out_of_stock").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].field, "stock");
    assert!(result[0].negated);
}

#[test]
fn test_negation_minus() {
    let result = parse_dsl("-country:CN").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].field, "country");
    assert!(result[0].negated);
}

#[test]
fn test_implicit_and() {
    let result = parse_dsl("type:gpu price:<=100 country:US").unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].field, "type");
    assert_eq!(result[1].field, "price");
    assert_eq!(result[2].field, "country");
}

#[test]
fn test_explicit_and() {
    let result = parse_dsl("type:gpu AND price:<=100").unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn test_complex_query() {
    let result = parse_dsl("type:(gpu OR compute) price:[50 TO 500] cores:>=8 !stock:out_of_stock")
        .unwrap();
    assert_eq!(result.len(), 4);

    // First filter: type:(gpu OR compute)
    assert_eq!(result[0].field, "type");
    assert_eq!(result[0].values.len(), 2);
    assert!(!result[0].negated);

    // Second filter: price:[50 TO 500]
    assert_eq!(result[1].field, "price");
    assert_eq!(result[1].operator, Operator::Range);
    assert_eq!(result[1].values.len(), 2);

    // Third filter: cores:>=8
    assert_eq!(result[2].field, "cores");
    assert_eq!(result[2].operator, Operator::Gte);

    // Fourth filter: !stock:out_of_stock
    assert_eq!(result[3].field, "stock");
    assert!(result[3].negated);
}

#[test]
fn test_value_types() {
    let result = parse_dsl("price:99.99 cores:8 unmetered:true name:test").unwrap();
    assert_eq!(result.len(), 4);

    match &result[0].values[0] {
        Value::Number(n) => assert_eq!(*n, 99.99),
        _ => panic!("Expected Number"),
    }

    match &result[1].values[0] {
        Value::Integer(i) => assert_eq!(*i, 8),
        _ => panic!("Expected Integer"),
    }

    match &result[2].values[0] {
        Value::Boolean(b) => assert!(*b),
        _ => panic!("Expected Boolean"),
    }

    match &result[3].values[0] {
        Value::String(s) => assert_eq!(s, "test"),
        _ => panic!("Expected String"),
    }
}

#[test]
fn test_field_with_underscore() {
    let result = parse_dsl("gpu_count:>=2").unwrap();
    assert_eq!(result[0].field, "gpu_count");
}

#[test]
fn test_value_with_underscore() {
    let result = parse_dsl("stock:out_of_stock").unwrap();
    assert_eq!(result[0].values, vec![Value::String("out_of_stock".to_string())]);
}

#[test]
fn test_empty_query() {
    let result = parse_dsl("").unwrap();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_whitespace_handling() {
    let result = parse_dsl("  type:gpu   price:>=100  ").unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn test_error_missing_field() {
    let result = parse_dsl(":value");
    assert!(result.is_err());
}

#[test]
fn test_error_missing_colon() {
    let result = parse_dsl("type gpu");
    assert!(result.is_err());
}

#[test]
fn test_error_unclosed_paren() {
    let result = parse_dsl("type:(gpu OR compute");
    assert!(result.is_err());
}

#[test]
fn test_error_unclosed_bracket() {
    let result = parse_dsl("price:[50 TO 100");
    assert!(result.is_err());
}

#[test]
fn test_error_missing_to_in_range() {
    let result = parse_dsl("price:[50 100]");
    assert!(result.is_err());
}

#[test]
fn test_error_invalid_character() {
    let result = parse_dsl("type:gpu @ price:100");
    assert!(result.is_err());
}

#[test]
fn test_multiple_or_in_group() {
    let result = parse_dsl("type:(gpu OR compute OR dedicated)").unwrap();
    assert_eq!(result[0].values.len(), 3);
}

#[test]
fn test_float_value() {
    let result = parse_dsl("price:123.45").unwrap();
    match &result[0].values[0] {
        Value::Number(n) => assert_eq!(*n, 123.45),
        _ => panic!("Expected Number"),
    }
}

#[test]
fn test_boolean_false() {
    let result = parse_dsl("unmetered:false").unwrap();
    match &result[0].values[0] {
        Value::Boolean(b) => assert!(!*b),
        _ => panic!("Expected Boolean"),
    }
}

#[test]
fn test_case_insensitive_keywords() {
    let result = parse_dsl("type:(gpu or compute) and price:[50 to 100]").unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].values.len(), 2);
    assert_eq!(result[1].operator, Operator::Range);
}

#[test]
fn test_negated_or_group() {
    let result = parse_dsl("!type:(gpu OR compute)").unwrap();
    assert_eq!(result.len(), 1);
    assert!(result[0].negated);
    assert_eq!(result[0].values.len(), 2);
}

#[test]
fn test_field_with_dot() {
    let result = parse_dsl("gpu.name:RTX4090").unwrap();
    assert_eq!(result[0].field, "gpu.name");
}
