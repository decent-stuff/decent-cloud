use super::types::{Operator, Value};
use super::{parse_dsl, SqlValue};
use crate::search::builder::build_sql;

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
    let result =
        parse_dsl("type:(gpu OR compute) price:[50 TO 500] cores:>=8 !stock:out_of_stock").unwrap();
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
    assert_eq!(
        result[0].values,
        vec![Value::String("out_of_stock".to_string())]
    );
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
    assert_eq!(sql, "product_type = $1");
    assert_eq!(values, vec![SqlValue::String("gpu".to_string())]);
}

#[test]
fn test_sql_gte_operator() {
    let filters = parse_dsl("price:>=100").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "monthly_price >= $1");
    assert_eq!(values, vec![SqlValue::Float(100.0)]);
}

#[test]
fn test_sql_lte_operator() {
    let filters = parse_dsl("price:<=500").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "monthly_price <= $1");
    assert_eq!(values, vec![SqlValue::Float(500.0)]);
}

#[test]
fn test_sql_gt_operator() {
    let filters = parse_dsl("cores:>8").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "processor_cores > $1");
    assert_eq!(values, vec![SqlValue::Integer(8)]);
}

#[test]
fn test_sql_lt_operator() {
    let filters = parse_dsl("cores:<16").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "processor_cores < $1");
    assert_eq!(values, vec![SqlValue::Integer(16)]);
}

#[test]
fn test_sql_range() {
    let filters = parse_dsl("price:[50 TO 200]").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "(monthly_price >= $1 AND monthly_price <= $2)");
    assert_eq!(values, vec![SqlValue::Float(50.0), SqlValue::Float(200.0)]);
}

#[test]
fn test_sql_or_group() {
    let filters = parse_dsl("type:(gpu OR compute)").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "(product_type = $1 OR product_type = $2)");
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
    assert_eq!(
        sql,
        "(datacenter_country = $1 OR datacenter_country = $2 OR datacenter_country = $3)"
    );
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
    assert_eq!(sql, "stock_status != $1");
    assert_eq!(values, vec![SqlValue::String("out_of_stock".to_string())]);
}

#[test]
fn test_sql_negation_gte() {
    let filters = parse_dsl("!price:>=100").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "monthly_price < $1");
    assert_eq!(values, vec![SqlValue::Float(100.0)]);
}

#[test]
fn test_sql_negation_or_group() {
    let filters = parse_dsl("!type:(gpu OR compute)").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "NOT (product_type = $1 OR product_type = $2)");
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
    assert_eq!(sql, "NOT (monthly_price >= $1 AND monthly_price <= $2)");
    assert_eq!(values, vec![SqlValue::Float(50.0), SqlValue::Float(200.0)]);
}

#[test]
fn test_sql_text_like_name() {
    let filters = parse_dsl("name:server").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "offer_name LIKE $1");
    assert_eq!(values, vec![SqlValue::String("%server%".to_string())]);
}

#[test]
fn test_sql_text_like_gpu() {
    let filters = parse_dsl("gpu:RTX").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "gpu_name LIKE $1");
    assert_eq!(values, vec![SqlValue::String("%RTX%".to_string())]);
}

#[test]
fn test_sql_text_like_memory() {
    let filters = parse_dsl("memory:32GB").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "memory_amount LIKE $1");
    assert_eq!(values, vec![SqlValue::String("%32GB%".to_string())]);
}

#[test]
fn test_sql_text_like_features() {
    let filters = parse_dsl("features:raid").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "features LIKE $1");
    assert_eq!(values, vec![SqlValue::String("%raid%".to_string())]);
}

#[test]
fn test_sql_text_like_negated() {
    let filters = parse_dsl("!name:test").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "offer_name NOT LIKE $1");
    assert_eq!(values, vec![SqlValue::String("%test%".to_string())]);
}

#[test]
fn test_sql_boolean_true() {
    let filters = parse_dsl("unmetered:true").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "unmetered_bandwidth = $1");
    assert_eq!(values, vec![SqlValue::Bool(true)]);
}

#[test]
fn test_sql_boolean_false() {
    let filters = parse_dsl("unmetered:false").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "unmetered_bandwidth = $1");
    assert_eq!(values, vec![SqlValue::Bool(false)]);
}

#[test]
fn test_sql_float_price() {
    let filters = parse_dsl("price:99.99").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "monthly_price = $1");
    assert_eq!(values, vec![SqlValue::Float(99.99)]);
}

#[test]
fn test_sql_integer_to_float_conversion() {
    let filters = parse_dsl("price:100").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(sql, "monthly_price = $1");
    assert_eq!(values, vec![SqlValue::Float(100.0)]);
}

#[test]
fn test_sql_multiple_filters_and() {
    let filters = parse_dsl("type:gpu price:>=100 country:US").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();
    assert_eq!(
        sql,
        "product_type = $1 AND monthly_price >= $2 AND datacenter_country = $3"
    );
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
    let filters =
        parse_dsl("type:(gpu OR compute) price:[50 TO 500] cores:>=8 !stock:out_of_stock").unwrap();
    let (sql, values) = build_sql(&filters).unwrap();

    assert_eq!(
        sql,
        "(product_type = $1 OR product_type = $2) AND (monthly_price >= $3 AND monthly_price <= $4) AND processor_cores >= $5 AND stock_status != $6"
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
        // Basic offering info
        ("name:test", "offer_name LIKE $1"),
        ("desc:cloud", "description LIKE $1"),
        ("type:gpu", "product_type = $1"),
        ("stock:available", "stock_status = $1"),
        ("source:seeded", "offering_source = $1"),
        // Pricing
        ("price:100", "monthly_price = $1"),
        ("setup_fee:50", "setup_fee = $1"),
        ("currency:EUR", "currency = $1"),
        // Processor
        ("cores:8", "processor_cores = $1"),
        ("cpu_count:2", "processor_amount = $1"),
        ("cpu_brand:AMD", "processor_brand LIKE $1"),
        ("cpu_speed:3.5GHz", "processor_speed LIKE $1"),
        ("cpu:Xeon", "processor_name LIKE $1"),
        // Memory
        ("memory:32GB", "memory_amount LIKE $1"),
        ("mem_type:DDR5", "memory_type LIKE $1"),
        ("ecc:ECC", "memory_error_correction LIKE $1"),
        // Storage
        ("ssd:500GB", "total_ssd_capacity LIKE $1"),
        ("ssd_count:2", "ssd_amount = $1"),
        ("hdd:2TB", "total_hdd_capacity LIKE $1"),
        ("hdd_count:4", "hdd_amount = $1"),
        // Location
        ("country:US", "datacenter_country = $1"),
        ("city:NYC", "datacenter_city = $1"),
        // GPU
        ("gpu:RTX", "gpu_name LIKE $1"),
        ("gpu_count:2", "gpu_count = $1"),
        ("gpu_memory:16", "gpu_memory_gb = $1"),
        // Network
        ("unmetered:true", "unmetered_bandwidth = $1"),
        ("uplink:10Gbps", "uplink_speed LIKE $1"),
        ("traffic:1000", "traffic = $1"),
        // Contract terms
        ("min_hours:720", "min_contract_hours = $1"),
        ("max_hours:8760", "max_contract_hours = $1"),
        ("billing:monthly", "billing_interval = $1"),
        // Platform/features
        ("virt:kvm", "virtualization_type = $1"),
        ("panel:cPanel", "control_panel LIKE $1"),
        ("features:ssd", "features LIKE $1"),
        ("os:Ubuntu", "operating_systems LIKE $1"),
        ("payment:crypto", "payment_methods LIKE $1"),
        // Trust
        ("trust:95", "trust_score = $1"),
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
    let (sql, values): (String, Vec<SqlValue>) = build_sql(&[]).unwrap();
    assert_eq!(sql, "");
    assert_eq!(values.len(), 0);
}
