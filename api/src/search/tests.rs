use super::{build_sql, parse_dsl, SqlValue};
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

// SQL Builder Tests

#[test]
fn test_sql_simple_equality() {
    let filters = parse_dsl("type:gpu").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "product_type = ?");
    assert_eq!(values, vec![SqlValue::String("gpu".to_string())]);
}

#[test]
fn test_sql_gte_operator() {
    let filters = parse_dsl("price:>=100").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "monthly_price >= ?");
    assert_eq!(values, vec![SqlValue::Float(100.0)]);
}

#[test]
fn test_sql_lte_operator() {
    let filters = parse_dsl("price:<=500").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "monthly_price <= ?");
    assert_eq!(values, vec![SqlValue::Float(500.0)]);
}

#[test]
fn test_sql_gt_operator() {
    let filters = parse_dsl("cores:>8").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "processor_cores > ?");
    assert_eq!(values, vec![SqlValue::Integer(8)]);
}

#[test]
fn test_sql_lt_operator() {
    let filters = parse_dsl("cores:<16").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "processor_cores < ?");
    assert_eq!(values, vec![SqlValue::Integer(16)]);
}

#[test]
fn test_sql_range() {
    let filters = parse_dsl("price:[50 TO 200]").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "(monthly_price >= ? AND monthly_price <= ?)");
    assert_eq!(values, vec![SqlValue::Float(50.0), SqlValue::Float(200.0)]);
}

#[test]
fn test_sql_or_group() {
    let filters = parse_dsl("type:(gpu OR compute)").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "(product_type = ? OR product_type = ?)");
    assert_eq!(
        values,
        vec![
            SqlValue::String("gpu".to_string()),
            SqlValue::String("compute".to_string())
        ]
    );
}

#[test]
fn test_sql_or_group_three_values() {
    let filters = parse_dsl("country:(US OR DE OR NL)").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "(datacenter_country = ? OR datacenter_country = ? OR datacenter_country = ?)");
    assert_eq!(
        values,
        vec![
            SqlValue::String("US".to_string()),
            SqlValue::String("DE".to_string()),
            SqlValue::String("NL".to_string())
        ]
    );
}

#[test]
fn test_sql_negation_simple() {
    let filters = parse_dsl("!stock:out_of_stock").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "stock_status != ?");
    assert_eq!(values, vec![SqlValue::String("out_of_stock".to_string())]);
}

#[test]
fn test_sql_negation_gte() {
    let filters = parse_dsl("!price:>=100").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "monthly_price < ?");
    assert_eq!(values, vec![SqlValue::Float(100.0)]);
}

#[test]
fn test_sql_negation_or_group() {
    let filters = parse_dsl("!type:(gpu OR compute)").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "NOT (product_type = ? OR product_type = ?)");
    assert_eq!(
        values,
        vec![
            SqlValue::String("gpu".to_string()),
            SqlValue::String("compute".to_string())
        ]
    );
}

#[test]
fn test_sql_negation_range() {
    let filters = parse_dsl("!price:[50 TO 200]").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "NOT (monthly_price >= ? AND monthly_price <= ?)");
    assert_eq!(values, vec![SqlValue::Float(50.0), SqlValue::Float(200.0)]);
}

#[test]
fn test_sql_text_like_name() {
    let filters = parse_dsl("name:server").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "offer_name LIKE ?");
    assert_eq!(values, vec![SqlValue::String("%server%".to_string())]);
}

#[test]
fn test_sql_text_like_gpu() {
    let filters = parse_dsl("gpu:RTX").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "gpu_name LIKE ?");
    assert_eq!(values, vec![SqlValue::String("%RTX%".to_string())]);
}

#[test]
fn test_sql_text_like_memory() {
    let filters = parse_dsl("memory:32GB").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "memory_amount LIKE ?");
    assert_eq!(values, vec![SqlValue::String("%32GB%".to_string())]);
}

#[test]
fn test_sql_text_like_features() {
    let filters = parse_dsl("features:raid").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "features LIKE ?");
    assert_eq!(values, vec![SqlValue::String("%raid%".to_string())]);
}

#[test]
fn test_sql_text_like_negated() {
    let filters = parse_dsl("!name:test").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "offer_name NOT LIKE ?");
    assert_eq!(values, vec![SqlValue::String("%test%".to_string())]);
}

#[test]
fn test_sql_boolean_true() {
    let filters = parse_dsl("unmetered:true").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "unmetered_bandwidth = ?");
    assert_eq!(values, vec![SqlValue::Bool(true)]);
}

#[test]
fn test_sql_boolean_false() {
    let filters = parse_dsl("unmetered:false").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "unmetered_bandwidth = ?");
    assert_eq!(values, vec![SqlValue::Bool(false)]);
}

#[test]
fn test_sql_float_price() {
    let filters = parse_dsl("price:99.99").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "monthly_price = ?");
    assert_eq!(values, vec![SqlValue::Float(99.99)]);
}

#[test]
fn test_sql_integer_to_float_conversion() {
    let filters = parse_dsl("price:100").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "monthly_price = ?");
    assert_eq!(values, vec![SqlValue::Float(100.0)]);
}

#[test]
fn test_sql_multiple_filters_and() {
    let filters = parse_dsl("type:gpu price:>=100 country:US").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "product_type = ? AND monthly_price >= ? AND datacenter_country = ?");
    assert_eq!(
        values,
        vec![
            SqlValue::String("gpu".to_string()),
            SqlValue::Float(100.0),
            SqlValue::String("US".to_string())
        ]
    );
}

#[test]
fn test_sql_complex_query() {
    let filters = parse_dsl("type:(gpu OR compute) price:[50 TO 500] cores:>=8 !stock:out_of_stock")
        .unwrap();
    let (sql, values) = build_sql(&filters).unwrap();

    assert_eq!(
        sql,
        "(product_type = ? OR product_type = ?) AND (monthly_price >= ? AND monthly_price <= ?) AND processor_cores >= ? AND stock_status != ?"
    );

    assert_eq!(
        values,
        vec![
            SqlValue::String("gpu".to_string()),
            SqlValue::String("compute".to_string()),
            SqlValue::Float(50.0),
            SqlValue::Float(500.0),
            SqlValue::Integer(8),
            SqlValue::String("out_of_stock".to_string())
        ]
    );
}

#[test]
fn test_sql_all_allowlisted_fields() {
    let queries = vec![
        ("name:test", "offer_name LIKE ?"),
        ("type:gpu", "product_type = ?"),
        ("stock:available", "stock_status = ?"),
        ("price:100", "monthly_price = ?"),
        ("cores:8", "processor_cores = ?"),
        ("memory:32GB", "memory_amount LIKE ?"),
        ("country:US", "datacenter_country = ?"),
        ("city:NYC", "datacenter_city = ?"),
        ("gpu:RTX", "gpu_name LIKE ?"),
        ("gpu_count:2", "gpu_count = ?"),
        ("gpu_memory:16", "gpu_memory_gb = ?"),
        ("features:ssd", "features LIKE ?"),
        ("unmetered:true", "unmetered_bandwidth = ?"),
        ("traffic:1000", "traffic = ?"),
        ("trust:95", "trust_score = ?"),
    ];

    for (query, expected_sql) in queries {
        let filters = parse_dsl(query).unwrap();
        let (sql, _) = build_sql(&filters).unwrap();
        assert_eq!(sql, expected_sql, "Failed for query: {}", query);
    }
}

#[test]
fn test_sql_unknown_field_error() {
    let filters = parse_dsl("invalid_field:value").unwrap();
    let result = build_sql(&filters);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown field: invalid_field"));
}

#[test]
fn test_sql_empty_filters() {
    let (sql, values) = build_sql(&[]).unwrap();
    assert_eq!(sql, "");
    assert_eq!(values.len(), 0);
}
