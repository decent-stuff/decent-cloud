use parse_size::parse_size;
use regex::Regex;
use serde_json::Value;
use std::str::FromStr;
use strsim::jaro_winkler;

#[derive(Debug, Clone)]
pub enum Search {
    Compare {
        key: String,
        op: CompareOp,
        value: Value,
    },
    And(Box<Search>, Box<Search>),
    Or(Box<Search>, Box<Search>),
}

impl FromStr for Search {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Handle parentheses for grouping (optional enhancement)
        let s = if s.starts_with('(') && s.ends_with(')') {
            &s[1..s.len() - 1]
        } else {
            s
        };

        // Parse 'and' and 'or' operators with basic precedence
        if let Some(index) = s.find(" and ") {
            let (left, right) = s.split_at(index);
            let right = &right[5..]; // Skip ' and '
            let left = Search::from_str(left)?;
            let right = Search::from_str(right)?;
            Ok(Search::And(Box::new(left), Box::new(right)))
        } else if let Some(index) = s.find(" or ") {
            let (left, right) = s.split_at(index);
            let right = &right[4..]; // Skip ' or '
            let left = Search::from_str(left)?;
            let right = Search::from_str(right)?;
            Ok(Search::Or(Box::new(left), Box::new(right)))
        } else {
            let re = Regex::new(
                r"^\s*([\w\.\-_+]+)\s*(==|!=|like|ilike|~|notlike|>=|<=|>|<|=|regex|re|matches|contains|startswith|endswith)\s*(.*)$",
            )
            .unwrap();
            if let Some(captures) = re.captures(s) {
                let key = captures[1].trim().to_string();
                let op = CompareOp::from_str(captures[2].trim())?;
                let value_str = captures[3]
                    .trim()
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_string();
                let value = serde_json::from_str(&value_str).unwrap_or(Value::String(value_str));
                Ok(Search::Compare { key, op, value })
            } else {
                anyhow::bail!(
                    "Expected format: `key op value` (with optional spaces around the op), found {}",
                    s
                );
            }
        }
    }
}

impl Search {
    pub fn matches(&self, json_value: &Value) -> bool {
        match self {
            Search::Compare { key, op, value } => find_and_compare(json_value, key, op, value),
            Search::And(left, right) => left.matches(json_value) && right.matches(json_value),
            Search::Or(left, right) => left.matches(json_value) || right.matches(json_value),
        }
    }
}

fn find_and_compare(json: &Value, key: &str, op: &CompareOp, value: &Value) -> bool {
    match json {
        Value::Object(map) => {
            for (k, v) in map {
                // Check if the current key matches
                if k == key && op.matches(v, value) {
                    return true;
                }
                // If key contains '.', attempt to navigate nested objects
                if key.contains('.') {
                    if let Some((first_part, rest_key)) = key.split_once('.') {
                        if k == first_part && find_and_compare(v, rest_key, op, value) {
                            return true;
                        }
                    }
                }
                // Recurse into the value
                if find_and_compare(v, key, op, value) {
                    return true;
                }
            }
            false
        }
        Value::Array(arr) => {
            for v in arr {
                if find_and_compare(v, key, op, value) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

impl std::fmt::Display for Search {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Search::Compare { key, op, value } => write!(f, "{} {} {}", key, op, value),
            Search::And(left, right) => write!(f, "({} AND {})", left, right),
            Search::Or(left, right) => write!(f, "({} OR {})", left, right),
        }
    }
}
#[derive(Debug, Clone)]
pub enum CompareOp {
    Equal,
    NotEqual,
    Like,
    NotLike,
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
            "like" | "ilike" | "~" => Ok(CompareOp::Like),
            "notlike" => Ok(CompareOp::NotLike),
            "gt" | ">" => Ok(CompareOp::GreaterThan),
            "lt" | "<" => Ok(CompareOp::LessThan),
            "ge" | ">=" => Ok(CompareOp::GreaterThanOrEqual),
            "le" | "<=" => Ok(CompareOp::LessThanOrEqual),
            "regex" | "re" | "matches" | "re~" => Ok(CompareOp::Regex),
            "contains" => Ok(CompareOp::Contains),
            "startswith" => Ok(CompareOp::StartsWith),
            "endswith" => Ok(CompareOp::EndsWith),
            _ => anyhow::bail!("Invalid comparison operator: {}", s),
        }
    }
}

impl std::fmt::Display for CompareOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompareOp::Equal => write!(f, "=="),
            CompareOp::NotEqual => write!(f, "!="),
            CompareOp::Like => write!(f, "like"),
            CompareOp::NotLike => write!(f, "notlike"),
            CompareOp::GreaterThan => write!(f, ">"),
            CompareOp::LessThan => write!(f, "<"),
            CompareOp::GreaterThanOrEqual => write!(f, ">="),
            CompareOp::LessThanOrEqual => write!(f, "<="),
            CompareOp::Regex => write!(f, "regex"),
            CompareOp::Contains => write!(f, "contains"),
            CompareOp::StartsWith => write!(f, "startswith"),
            CompareOp::EndsWith => write!(f, "endswith"),
        }
    }
}

fn try_parse_size_bytes(s: &Value) -> Value {
    if let Value::String(s) = s {
        match parse_size(s) {
            Ok(size) => Value::Number(serde_json::Number::from(size)),
            Err(_) => s.clone().into(),
        }
    } else {
        s.clone()
    }
}

impl CompareOp {
    fn matches(&self, value: &Value, other: &Value) -> bool {
        // Try to parse values as humanized bytes if possible (e.g., "100 MB" -> 100 * 1024 * 1024)
        let value = &try_parse_size_bytes(value);
        let other = &try_parse_size_bytes(other);

        match self {
            CompareOp::Equal => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
                (Value::String(a), Value::String(b)) => a.to_lowercase() == b.to_lowercase(),
                (a, b) => a == b,
            },
            CompareOp::NotEqual => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() != b.as_f64(),
                (Value::String(a), Value::String(b)) => a.to_lowercase() != b.to_lowercase(),
                (a, b) => a != b,
            },
            CompareOp::Like => match (value, other) {
                (Value::Number(a), Value::Number(b)) => {
                    a.as_f64() > b.as_f64().map(|b| b * 0.9)
                        && a.as_f64() < b.as_f64().map(|b| b * 1.1)
                }
                (Value::String(a), Value::String(b)) => {
                    jaro_winkler(&a.to_lowercase(), &b.to_lowercase()) > 0.9
                }
                _ => false,
            },
            CompareOp::NotLike => match (value, other) {
                (Value::Number(a), Value::Number(b)) => {
                    a.as_f64() <= b.as_f64().map(|b| b * 0.9)
                        || a.as_f64() >= b.as_f64().map(|b| b * 1.1)
                }
                (Value::String(a), Value::String(b)) => {
                    jaro_winkler(&a.to_lowercase(), &b.to_lowercase()) <= 0.9
                }
                _ => false,
            },
            CompareOp::GreaterThan => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() > b.as_f64(),
                (Value::String(a), Value::String(b)) => a.to_lowercase() > b.to_lowercase(),
                _ => false,
            },
            CompareOp::LessThan => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() < b.as_f64(),
                (Value::String(a), Value::String(b)) => a.to_lowercase() < b.to_lowercase(),
                _ => false,
            },
            CompareOp::GreaterThanOrEqual => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() >= b.as_f64(),
                (Value::String(a), Value::String(b)) => a.to_lowercase() >= b.to_lowercase(),
                _ => false,
            },
            CompareOp::LessThanOrEqual => match (value, other) {
                (Value::Number(a), Value::Number(b)) => a.as_f64() <= b.as_f64(),
                (Value::String(a), Value::String(b)) => a.to_lowercase() <= b.to_lowercase(),
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
                        s.to_lowercase().contains(&other.to_lowercase())
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
                        s.to_lowercase().starts_with(&other.to_lowercase())
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
                        s.to_lowercase().ends_with(&other.to_lowercase())
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

pub fn value_matches(json_value: &Value, search_str: &str) -> bool {
    let search = match Search::from_str(search_str) {
        Ok(search) => search,
        Err(e) => {
            println!("Failed to parse search string: {}", e);
            return false;
        }
    };
    search.matches(json_value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;
    use serde_json::Value;

    #[test]
    fn test_simple_key() {
        let json_str = r#"{"key1": "value1", "key2": "value2"}"#;
        let json_value: Value = from_str(json_str).unwrap();
        let search_str = "key1 = value1";
        let search = Search::from_str(search_str).unwrap();
        assert!(search.matches(&json_value));
    }

    #[test]
    fn test_match_like() {
        let json_str = r#"{"key1": "value1", "key2": "value2"}"#;
        let json_value: Value = from_str(json_str).unwrap();
        let search_str = "key1 like value1";
        let search = Search::from_str(search_str).unwrap();
        assert!(search.matches(&json_value));
        let search_str = "key1 like valuee1";
        let search = Search::from_str(search_str).unwrap();
        assert!(search.matches(&json_value));
    }

    #[test]
    fn test_nested_key() {
        let json_str = r#"{"parent": {"child": {"grandchild": "value"}}}"#;
        let json_value: Value = from_str(json_str).unwrap();
        let search_str = "parent.child.grandchild = value";
        let search = Search::from_str(search_str).unwrap();
        assert!(search.matches(&json_value));
    }

    #[test]
    fn test_nonexistent_key() {
        let json_str = r#"
            {
                "key1": "value1",
                "key2": "value2"
            }
        "#;
        let json_value: Value = from_str(json_str).unwrap();
        let search_str = "key3 = value3";
        let search = Search::from_str(search_str).unwrap();
        assert!(!search.matches(&json_value));
    }

    #[test]
    fn test_in_sequence() {
        let json_str = r#"
            [
                {"key1": "value1", "key2": "value2"},
                {"key3": "value3", "key4": "value4"}
            ]
        "#;
        let json_value: Value = from_str(json_str).unwrap();
        let search_str = "key3 = value3";
        let search = Search::from_str(search_str).unwrap();
        if let Value::Array(arr) = json_value {
            let matched = arr.iter().any(|v| search.matches(v));
            assert!(matched);
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_partial_nested_key() {
        let json_str = r#"
            {
                "parent": {
                    "child1": {"grandchild": "value1"},
                    "child2": {"grandchild": "value2"}
                }
            }
        "#;
        let json_value: Value = from_str(json_str).unwrap();
        let search_str = "parent.child2.grandchild = value2";
        let search = Search::from_str(search_str).unwrap();
        assert!(search.matches(&json_value));
    }

    #[test]
    fn test_comparison_operator() {
        let json_str = r#"
            {
                "stats": {
                    "count": 10
                }
            }
        "#;
        let json_value: Value = from_str(json_str).unwrap();
        let search_str = "stats.count > 5";
        let search = Search::from_str(search_str).unwrap();
        assert!(search.matches(&json_value));
    }

    #[test]
    fn test_and_operator() {
        let json_str = r#"{"key1": "memory-optimized", "key2": "GenericCloudService"}"#;
        let json_value: Value = from_str(json_str).unwrap();
        let search_str_hit = "key1 = memory-optimized and key2 = GenericCloudService";
        let search_str_miss = "key1 = storage-optimized and key2 = GenericCloudService";
        let search_hit = Search::from_str(search_str_hit).unwrap();
        let search_miss = Search::from_str(search_str_miss).unwrap();
        assert!(search_hit.matches(&json_value));
        assert!(!search_miss.matches(&json_value));
    }

    #[test]
    fn test_or_operator() {
        let json_str = r#"{"key1": 10, "key2": 5}"#;
        let json_value: Value = from_str(json_str).unwrap();
        let search_str = "key1 = 10 or key2 = 10";
        let search = Search::from_str(search_str).unwrap();
        assert!(search.matches(&json_value));
    }

    #[test]
    fn test_deep_nested_match() {
        let json_str = r#"
            {
                "level1": {
                    "level2": {
                        "level3": {
                            "key": "value",
                            "name": "deep_value",
                            "otherkey": "other_value"
                        }
                    }
                }
            }
        "#;
        let json_value: Value = from_str(json_str).unwrap();
        let search = Search::from_str("name = deep_value").unwrap();
        assert!(search.matches(&json_value));
        let search = Search::from_str("name = deep_value and otherkey = other_value").unwrap();
        assert!(search.matches(&json_value));
    }

    #[test]
    fn test_key_with_dot_notation() {
        let json_str = r#"
            {
                "level1": {
                    "level2": {
                        "name.with.dot": "value_with_dot"
                    }
                }
            }
        "#;
        let json_value: Value = from_str(json_str).unwrap();
        let search = Search::from_str("level1.level2.name.with.dot = value_with_dot").unwrap();
        assert!(search.matches(&json_value));
        let search = Search::from_str("level2.name.with.dot = value_with_dot").unwrap();
        assert!(search.matches(&json_value));
        let search = Search::from_str("name.with.dot = value_with_dot").unwrap();
        assert!(search.matches(&json_value));
    }
}
