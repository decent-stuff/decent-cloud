use parse_size::parse_size;
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
        let re = Regex::new(r"^\s*([\w\.\-_+]+)\s*\b(.+?)\b\s*(.*)$").unwrap();

        if let Some(captures) = re.captures(s) {
            Ok(Search::new(
                &captures[1].trim(),
                &captures[2].trim(),
                &captures[3].trim(),
            ))
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

fn try_parse_size_bytes(s: &Value) -> Value {
    if let Value::String(s) = s {
        match parse_size(s) {
            Ok(size) => Value::Number(serde_yaml_ng::Number::from(size)),
            Err(_) => s.clone().into(),
        }
    } else {
        s.clone()
    }
}

impl CompareOp {
    fn matches(&self, value: &Value, other: &Value) -> bool {
        // Try to parse values as humanized bytes if possible (eg. "100 MB" -> 100 * 1024 * 1024)
        let value = &try_parse_size_bytes(value);
        let other = &try_parse_size_bytes(other);
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

/// Recursively searches a YAML structure for a key and compares its value.
///
/// # Arguments
///
/// * `yaml_value` - The YAML value to search.
/// * `key` - The key to search for, which may be nested (e.g., "key1.key2.key3").
/// * `value` - The value to compare against.
/// * `compare_op` - The comparison operator to use.
///
/// # Returns
///
/// * `true` if a matching key-value pair is found that satisfies the comparison operator.
/// * `false` otherwise.
fn _recursive_yaml_search(
    yaml_value: &Value,
    key: &str,
    value: &Value,
    compare_op: &CompareOp,
) -> bool {
    // Split the key into the first part and the rest for handling nested keys
    let (key_part1, key_part2) = match key.split_once('.') {
        Some((first, rest)) => (first, rest),
        None => (key, ""),
    };

    match yaml_value {
        Value::Mapping(map) => {
            // If the mapping contains key_part1
            if let Some(v) = map.get(key_part1) {
                if key_part2.is_empty() {
                    // No more parts in the key; compare the value
                    return compare_op.matches(v, value);
                } else {
                    // Recurse with the value at key_part1 and the rest of the key
                    return _recursive_yaml_search(v, key_part2, value, compare_op);
                }
            }
            // If key_part1 not found, recurse into all values in the map
            for (_, v) in map {
                if _recursive_yaml_search(v, key, value, compare_op) {
                    return true;
                }
            }
            false
        }
        Value::Sequence(seq) => {
            // Iterate over the elements in the sequence and recurse
            for v in seq {
                if _recursive_yaml_search(v, key, value, compare_op) {
                    return true;
                }
            }
            false
        }
        _ => false, // For scalar values, return false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml_ng::from_str;
    use serde_yaml_ng::Value;

    #[test]
    fn test_recursive_yaml_search_simple_key() {
        let yaml_str = r#"
            key1: value1
            key2: value2
        "#;
        let yaml_value: Value = from_str(yaml_str).unwrap();
        let key = "key1";
        let value = Value::String("value1".to_string());
        let compare_op = CompareOp::Equal;
        assert!(_recursive_yaml_search(
            &yaml_value,
            key,
            &value,
            &compare_op
        ));
    }

    #[test]
    fn test_recursive_yaml_search_nested_key() {
        let yaml_str = r#"
            parent:
              child:
                grandchild: value
        "#;
        let yaml_value: Value = from_str(yaml_str).unwrap();
        let key = "parent.child.grandchild";
        let value = Value::String("value".to_string());
        let compare_op = CompareOp::Equal;
        assert!(_recursive_yaml_search(
            &yaml_value,
            key,
            &value,
            &compare_op
        ));
    }

    #[test]
    fn test_recursive_yaml_search_nonexistent_key() {
        let yaml_str = r#"
            key1: value1
            key2: value2
        "#;
        let yaml_value: Value = from_str(yaml_str).unwrap();
        let key = "key3";
        let value = Value::String("value3".to_string());
        let compare_op = CompareOp::Equal;
        assert!(!_recursive_yaml_search(
            &yaml_value,
            key,
            &value,
            &compare_op
        ));
    }

    #[test]
    fn test_recursive_yaml_search_in_sequence() {
        let yaml_str = r#"
            - key1: value1
              key2: value2
            - key3: value3
              key4: value4
        "#;
        let yaml_value: Value = from_str(yaml_str).unwrap();
        let key = "key3";
        let value = Value::String("value3".to_string());
        let compare_op = CompareOp::Equal;
        assert!(_recursive_yaml_search(
            &yaml_value,
            key,
            &value,
            &compare_op
        ));
    }

    #[test]
    fn test_recursive_yaml_search_partial_nested_key() {
        let yaml_str = r#"
            parent:
              child1:
                grandchild: value1
              child2:
                grandchild: value2
        "#;
        let yaml_value: Value = from_str(yaml_str).unwrap();
        let key = "parent.child2.grandchild";
        let value = Value::String("value2".to_string());
        let compare_op = CompareOp::Equal;
        assert!(_recursive_yaml_search(
            &yaml_value,
            key,
            &value,
            &compare_op
        ));
    }

    #[test]
    fn test_recursive_yaml_search_with_comparison_operator() {
        let yaml_str = r#"
            stats:
              count: 10
        "#;
        let yaml_value: Value = from_str(yaml_str).unwrap();
        let key = "stats.count";
        let value = Value::Number(serde_yaml_ng::Number::from(5));
        let compare_op = CompareOp::GreaterThan;
        assert!(_recursive_yaml_search(
            &yaml_value,
            key,
            &value,
            &compare_op
        ));
    }
}
