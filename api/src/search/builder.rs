use super::types::{Filter, Operator, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
}

/// Field allowlist mapping DSL field names to database column names
fn field_allowlist() -> HashMap<&'static str, FieldConfig> {
    let mut map = HashMap::new();
    // Basic offering info
    map.insert("name", FieldConfig::new("offer_name", FieldType::Text));
    map.insert("desc", FieldConfig::new("description", FieldType::Text));
    map.insert("type", FieldConfig::new("product_type", FieldType::Text));
    map.insert("stock", FieldConfig::new("stock_status", FieldType::Text));
    map.insert(
        "source",
        FieldConfig::new("offering_source", FieldType::Text),
    );

    // Pricing
    map.insert("price", FieldConfig::new("monthly_price", FieldType::Float));
    map.insert("setup_fee", FieldConfig::new("setup_fee", FieldType::Float));
    map.insert("currency", FieldConfig::new("currency", FieldType::Text));

    // Processor
    map.insert("cores", FieldConfig::new("processor_cores", FieldType::Int));
    map.insert(
        "cpu_count",
        FieldConfig::new("processor_amount", FieldType::Int),
    );
    map.insert(
        "cpu_brand",
        FieldConfig::new("processor_brand", FieldType::Text),
    );
    map.insert(
        "cpu_speed",
        FieldConfig::new("processor_speed", FieldType::Text),
    );
    map.insert("cpu", FieldConfig::new("processor_name", FieldType::Text));

    // Memory
    map.insert("memory", FieldConfig::new("memory_amount", FieldType::Text));
    map.insert("mem_type", FieldConfig::new("memory_type", FieldType::Text));
    map.insert(
        "ecc",
        FieldConfig::new("memory_error_correction", FieldType::Text),
    );

    // Storage
    map.insert(
        "ssd",
        FieldConfig::new("total_ssd_capacity", FieldType::Text),
    );
    map.insert("ssd_count", FieldConfig::new("ssd_amount", FieldType::Int));
    map.insert(
        "hdd",
        FieldConfig::new("total_hdd_capacity", FieldType::Text),
    );
    map.insert("hdd_count", FieldConfig::new("hdd_amount", FieldType::Int));

    // Location
    map.insert(
        "country",
        FieldConfig::new("datacenter_country", FieldType::Text),
    );
    map.insert("city", FieldConfig::new("datacenter_city", FieldType::Text));

    // GPU
    map.insert("gpu", FieldConfig::new("gpu_name", FieldType::Text));
    map.insert("gpu_count", FieldConfig::new("gpu_count", FieldType::Int));
    map.insert(
        "gpu_memory",
        FieldConfig::new("gpu_memory_gb", FieldType::Int),
    );

    // Network
    map.insert(
        "unmetered",
        FieldConfig::new("unmetered_bandwidth", FieldType::Bool),
    );
    map.insert("uplink", FieldConfig::new("uplink_speed", FieldType::Text));
    map.insert("traffic", FieldConfig::new("traffic", FieldType::Int));

    // Contract terms
    map.insert(
        "min_hours",
        FieldConfig::new("min_contract_hours", FieldType::Int),
    );
    map.insert(
        "max_hours",
        FieldConfig::new("max_contract_hours", FieldType::Int),
    );
    map.insert(
        "billing",
        FieldConfig::new("billing_interval", FieldType::Text),
    );

    // Platform/features
    map.insert(
        "virt",
        FieldConfig::new("virtualization_type", FieldType::Text),
    );
    map.insert("panel", FieldConfig::new("control_panel", FieldType::Text));
    map.insert("features", FieldConfig::new("features", FieldType::Csv));
    map.insert("os", FieldConfig::new("operating_systems", FieldType::Csv));
    map.insert(
        "payment",
        FieldConfig::new("payment_methods", FieldType::Csv),
    );

    // Trust
    map.insert("trust", FieldConfig::new("trust_score", FieldType::Int));
    map
}

#[derive(Debug, Clone)]
struct FieldConfig {
    db_column: &'static str,
    field_type: FieldType,
}

#[derive(Debug, Clone, PartialEq)]
enum FieldType {
    Text,
    Int,
    Float,
    Bool,
    Csv,
}

impl FieldConfig {
    fn new(db_column: &'static str, field_type: FieldType) -> Self {
        Self {
            db_column,
            field_type,
        }
    }
}

/// Builds SQL WHERE clause and bind values from parsed DSL filters
/// Uses PostgreSQL-style numbered placeholders ($1, $2, etc.)
pub fn build_sql(filters: &[Filter]) -> Result<(String, Vec<SqlValue>), String> {
    build_sql_with_offset(filters, 0)
}

/// Builds SQL WHERE clause with a starting placeholder offset
/// Useful when combining with other query parts that already have placeholders
pub fn build_sql_with_offset(
    filters: &[Filter],
    start_offset: usize,
) -> Result<(String, Vec<SqlValue>), String> {
    if filters.is_empty() {
        return Ok((String::new(), Vec::new()));
    }

    let allowlist = field_allowlist();
    let mut sql_parts = Vec::new();
    let mut bind_values = Vec::new();
    let mut placeholder_idx = start_offset;

    for filter in filters {
        let config = allowlist
            .get(filter.field.as_str())
            .ok_or_else(|| format!("Unknown field: {}", filter.field))?;

        let (sql, values) = build_filter_sql(filter, config, &mut placeholder_idx)?;
        sql_parts.push(sql);
        bind_values.extend(values);
    }

    let sql = sql_parts.join(" AND ");
    Ok((sql, bind_values))
}

/// Fields that should use LIKE matching for substring search
const LIKE_FIELDS: &[&str] = &[
    "name",
    "desc",
    "memory",
    "gpu",
    "features",
    "os",
    "ssd",
    "hdd",
    "cpu_brand",
    "cpu_speed",
    "cpu",
    "mem_type",
    "ecc",
    "uplink",
    "panel",
    "payment",
];

fn build_filter_sql(
    filter: &Filter,
    config: &FieldConfig,
    placeholder_idx: &mut usize,
) -> Result<(String, Vec<SqlValue>), String> {
    let column = config.db_column;
    let use_like = (matches!(config.field_type, FieldType::Text)
        || matches!(config.field_type, FieldType::Csv))
        && matches!(filter.operator, Operator::Eq)
        && LIKE_FIELDS.contains(&filter.field.as_str());

    let sql = match (&filter.operator, filter.values.len()) {
        (Operator::Eq, 1) if use_like => build_like_clause(column, filter.negated, placeholder_idx),
        (Operator::Eq, 1) => build_comparison_clause(column, "=", filter.negated, placeholder_idx),
        (Operator::Eq, n) if n > 1 => build_or_group(column, n, filter.negated, placeholder_idx),
        (Operator::Gte, 1) => build_comparison_clause(column, ">=", filter.negated, placeholder_idx),
        (Operator::Lte, 1) => build_comparison_clause(column, "<=", filter.negated, placeholder_idx),
        (Operator::Gt, 1) => build_comparison_clause(column, ">", filter.negated, placeholder_idx),
        (Operator::Lt, 1) => build_comparison_clause(column, "<", filter.negated, placeholder_idx),
        (Operator::Range, 2) => build_range_clause(column, filter.negated, placeholder_idx),
        _ => {
            return Err(format!(
                "Invalid operator/value combination for field: {}",
                filter.field
            ))
        }
    };

    let values = convert_values(&filter.values, &config.field_type, use_like)?;
    Ok((sql, values))
}

fn build_comparison_clause(column: &str, op: &str, negated: bool, idx: &mut usize) -> String {
    *idx += 1;
    if negated {
        let inverse_op = match op {
            "=" => "!=",
            ">=" => "<",
            "<=" => ">",
            ">" => "<=",
            "<" => ">=",
            _ => op,
        };
        format!("{} {} ${}", column, inverse_op, *idx)
    } else {
        format!("{} {} ${}", column, op, *idx)
    }
}

fn build_like_clause(column: &str, negated: bool, idx: &mut usize) -> String {
    *idx += 1;
    if negated {
        format!("{} NOT LIKE ${}", column, *idx)
    } else {
        format!("{} LIKE ${}", column, *idx)
    }
}

fn build_or_group(column: &str, count: usize, negated: bool, idx: &mut usize) -> String {
    let placeholders: Vec<String> = (0..count)
        .map(|_| {
            *idx += 1;
            format!("{} = ${}", column, *idx)
        })
        .collect();
    let inner = placeholders.join(" OR ");

    if negated {
        format!("NOT ({})", inner)
    } else {
        format!("({})", inner)
    }
}

fn build_range_clause(column: &str, negated: bool, idx: &mut usize) -> String {
    *idx += 1;
    let p1 = *idx;
    *idx += 1;
    let p2 = *idx;
    let clause = format!("({} >= ${} AND {} <= ${})", column, p1, column, p2);
    if negated {
        format!("NOT {}", clause)
    } else {
        clause
    }
}

fn convert_values(
    values: &[Value],
    field_type: &FieldType,
    use_like: bool,
) -> Result<Vec<SqlValue>, String> {
    values
        .iter()
        .map(|v| convert_value(v, field_type, use_like))
        .collect()
}

fn convert_value(
    value: &Value,
    field_type: &FieldType,
    use_like: bool,
) -> Result<SqlValue, String> {
    match (value, field_type) {
        (Value::String(s), FieldType::Text | FieldType::Csv) if use_like => {
            Ok(SqlValue::String(format!("%{}%", s)))
        }
        (Value::String(s), FieldType::Text | FieldType::Csv) => Ok(SqlValue::String(s.clone())),
        (Value::Integer(i), FieldType::Int) => Ok(SqlValue::Integer(*i)),
        (Value::Number(f), FieldType::Float) => Ok(SqlValue::Float(*f)),
        (Value::Integer(i), FieldType::Float) => Ok(SqlValue::Float(*i as f64)),
        (Value::Boolean(b), FieldType::Bool) => Ok(SqlValue::Bool(*b)),
        _ => Err(format!(
            "Type mismatch: {:?} cannot be used with {:?}",
            value, field_type
        )),
    }
}
