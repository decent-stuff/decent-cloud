use regex::Regex;
use serde_yaml_ng::Value;
use std::str::FromStr;

pub struct Search {
    key: String,
    value: Value,
    compare_op: CompareOp,
}

impl Search {
    pub fn new(key: &str, compare_op: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: serde_yaml_ng::from_str(value)
                .unwrap_or_else(|_| Value::String(value.to_string())),
            compare_op: CompareOp::from_str(compare_op).expect(
                "Invalid compare op, supported: = != > < >= <= re contains startswith endswith",
            ),
        }
    }
}

impl FromStr for Search {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Regex for `key op value` with optional spaces around the op
        let re = Regex::new(r"^\s*(\w+)\s*\b(.+?)\b\s*(.*)$").unwrap();

        if let Some(captures) = re.captures(s) {
            Ok(Search::new(&captures[1], &captures[2], &captures[3]))
        } else {
            anyhow::bail!(
                "Expected format: `key op value` (with optional spaces around the op), found {}",
                s
            );
        }
    }
}

#[derive(Debug, Clone)]
pub enum CompareOp {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Regex,
    Contains,
    StartsWith,
    EndsWith,
}

impl FromStr for CompareOp {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "eq" | "=" | "==" => Ok(CompareOp::Equal),
            "ne" | "!=" => Ok(CompareOp::NotEqual),
            "gt" | ">" => Ok(CompareOp::GreaterThan),
            "lt" | "<" => Ok(CompareOp::LessThan),
            "ge" | ">=" => Ok(CompareOp::GreaterThanOrEqual),
            "le" | "<=" => Ok(CompareOp::LessThanOrEqual),
            "regex" | "re" | "matches" | "=~" => Ok(CompareOp::Regex),
            "contains" => Ok(CompareOp::Contains),
            "startswith" => Ok(CompareOp::StartsWith),
            "endswith" => Ok(CompareOp::EndsWith),
            _ => anyhow::bail!("Invalid comparison operator: {}", s),
        }
    }
}

impl CompareOp {
    fn matches(&self, value: &Value, other: &Value) -> bool {
        match self {
            CompareOp::Equal => value == other,
            CompareOp::NotEqual => value != other,
            CompareOp::GreaterThan => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() > b.as_f64(),
                (Value::String(a), Value::String(b)) => a > b,
                _ => false,
            },
            CompareOp::LessThan => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() < b.as_f64(),
                (Value::String(a), Value::String(b)) => a < b,
                _ => false,
            },
            CompareOp::GreaterThanOrEqual => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() >= b.as_f64(),
                (Value::String(a), Value::String(b)) => a >= b,
                _ => false,
            },
            CompareOp::LessThanOrEqual => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() <= b.as_f64(),
                (Value::String(a), Value::String(b)) => a <= b,
                _ => false,
            },
            CompareOp::Regex => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other {
                        let re = Regex::new(other).unwrap();
                        re.is_match(s)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CompareOp::Contains => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other {
                        s.contains(other)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CompareOp::StartsWith => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other {
                        s.starts_with(other)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CompareOp::EndsWith => {
                if let Value::String(s) = value {
                    if let Value::String(other) = other {
                        s.ends_with(other)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }
}

pub fn yaml_value_matches(yaml_value: &Value, search_str: &str) -> bool {
    let search = match Search::from_str(search_str) {
        Ok(search) => search,
        Err(e) => {
            println!("Failed to parse search string: {}", e);
            return false;
        }
    };

    _recursive_yaml_search(yaml_value, &search.key, &search.value, &search.compare_op)
}

fn _recursive_yaml_search(
    yaml_value: &Value,
    key: &str,
    value: &Value,
    compare_op: &CompareOp,
) -> bool {
    match yaml_value {
        Value::Mapping(map) => {
            // YAML sequence in which the keys and values are both `serde_yaml_ng::Value`
            if let Some(v) = map.get(key) {
                return compare_op.matches(v, value);
            }

            if map
                .iter()
                .any(|(_, v)| _recursive_yaml_search(v, key, value, compare_op))
            {
                return true;
            }

            false
        }
        Value::Sequence(arr) => {
            // YAML sequence in which the elements are `serde_yaml_ng::Value`
            if arr
                .iter()
                .any(|v| _recursive_yaml_search(v, key, value, compare_op))
            {
                return true;
            }

            false
        }
        _ => false, // Since this is a string comparison, non-object and non-array values don't match
    }
}
