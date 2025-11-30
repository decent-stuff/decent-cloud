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
    map.insert("name", FieldConfig::new("offer_name", FieldType::Text));
    map.insert("type", FieldConfig::new("product_type", FieldType::Text));
    map.insert("stock", FieldConfig::new("stock_status", FieldType::Text));
    map.insert("price", FieldConfig::new("monthly_price", FieldType::Float));
    map.insert("cores", FieldConfig::new("processor_cores", FieldType::Int));
    map.insert("memory", FieldConfig::new("memory_amount", FieldType::Text));
    map.insert(
        "country",
        FieldConfig::new("datacenter_country", FieldType::Text),
    );
    map.insert("city", FieldConfig::new("datacenter_city", FieldType::Text));
    map.insert("gpu", FieldConfig::new("gpu_name", FieldType::Text));
    map.insert("gpu_count", FieldConfig::new("gpu_count", FieldType::Int));
    map.insert(
        "gpu_memory",
        FieldConfig::new("gpu_memory_gb", FieldType::Int),
    );
    map.insert("features", FieldConfig::new("features", FieldType::Csv));
    map.insert(
        "unmetered",
        FieldConfig::new("unmetered_bandwidth", FieldType::Bool),
    );
    map.insert("traffic", FieldConfig::new("traffic", FieldType::Int));
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
pub fn build_sql(filters: &[Filter]) -> Result<(String, Vec<SqlValue>), String> {
    if filters.is_empty() {
        return Ok((String::new(), Vec::new()));
    }

    let allowlist = field_allowlist();
    let mut sql_parts = Vec::new();
    let mut bind_values = Vec::new();

    for filter in filters {
        let config = allowlist
            .get(filter.field.as_str())
            .ok_or_else(|| format!("Unknown field: {}", filter.field))?;

        let (sql, values) = build_filter_sql(filter, config)?;
        sql_parts.push(sql);
        bind_values.extend(values);
    }

    let sql = sql_parts.join(" AND ");
    Ok((sql, bind_values))
}

fn build_filter_sql(
    filter: &Filter,
    config: &FieldConfig,
) -> Result<(String, Vec<SqlValue>), String> {
    let column = config.db_column;
    let use_like = (matches!(config.field_type, FieldType::Text)
        || matches!(config.field_type, FieldType::Csv))
        && matches!(filter.operator, Operator::Eq)
        && ["name", "memory", "gpu", "features"].contains(&filter.field.as_str());

    let sql = match (&filter.operator, filter.values.len()) {
        (Operator::Eq, 1) if use_like => build_like_clause(column, filter.negated),
        (Operator::Eq, 1) => build_comparison_clause(column, "=", filter.negated),
        (Operator::Eq, n) if n > 1 => build_or_group(column, n, filter.negated),
        (Operator::Gte, 1) => build_comparison_clause(column, ">=", filter.negated),
        (Operator::Lte, 1) => build_comparison_clause(column, "<=", filter.negated),
        (Operator::Gt, 1) => build_comparison_clause(column, ">", filter.negated),
        (Operator::Lt, 1) => build_comparison_clause(column, "<", filter.negated),
        (Operator::Range, 2) => build_range_clause(column, filter.negated),
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

fn build_comparison_clause(column: &str, op: &str, negated: bool) -> String {
    if negated {
        let inverse_op = match op {
            "=" => "!=",
            ">=" => "<",
            "<=" => ">",
            ">" => "<=",
            "<" => ">=",
            _ => op,
        };
        format!("{} {} ?", column, inverse_op)
    } else {
        format!("{} {} ?", column, op)
    }
}

fn build_like_clause(column: &str, negated: bool) -> String {
    if negated {
        format!("{} NOT LIKE ?", column)
    } else {
        format!("{} LIKE ?", column)
    }
}

fn build_or_group(column: &str, count: usize, negated: bool) -> String {
    let placeholders: Vec<String> = (0..count).map(|_| format!("{} = ?", column)).collect();
    let inner = placeholders.join(" OR ");

    if negated {
        format!("NOT ({})", inner)
    } else {
        format!("({})", inner)
    }
}

fn build_range_clause(column: &str, negated: bool) -> String {
    let clause = format!("({} >= ? AND {} <= ?)", column, column);
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
