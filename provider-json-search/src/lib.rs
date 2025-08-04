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

        // Handle parentheses for grouping (not completed/tested)
        let s = if s.starts_with('(') && s.ends_with(')') {
            &s[1..s.len() - 1]
        } else {
            s
        };
        let s_lower = s.to_lowercase();

        // Parse 'and' and 'or' operators with basic precedence
        if let Some(index) = s_lower.find(" and ") {
            let (left, right) = s.split_at(index);
            let right = &right[5..]; // Skip ' and '
            let left = Search::from_str(left)?;
            let right = Search::from_str(right)?;
            Ok(Search::And(Box::new(left), Box::new(right)))
        } else if let Some(index) = s_lower.find(" or ") {
            let (left, right) = s.split_at(index);
            let right = &right[4..]; // Skip ' or '
            let left = Search::from_str(left)?;
            let right = Search::from_str(right)?;
            Ok(Search::Or(Box::new(left), Box::new(right)))
        } else {
            let re = Regex::new(
                r"(?i)^\s*([\w\.\-_+]+)\s*(==|!=|like|ilike|~|notlike|>=|<=|>|<|=|regex|re|matches|contains|startswith|endswith)\s*(.*)$",
            )
            .unwrap();
            if let Some(captures) = re.captures(&s_lower) {
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
                if k.to_lowercase() == key.to_lowercase() && op.matches(v, value) {
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

fn try_parse_size_bytes(value: &Value) -> Value {
    let size_parser = parse_size::Config::new().with_byte_suffix(parse_size::ByteSuffix::Allow);
    if let Value::String(s) = value {
        match size_parser.parse_size(s) {
            Ok(size) => {
                if size > 0 {
                    Value::Number(serde_json::Number::from(size))
                } else {
                    Value::String(s.clone())
                }
            }
            Err(_) => Value::String(s.clone()),
        }
    } else {
        value.clone()
    }
}

impl CompareOp {
    fn matches(&self, value: &Value, other: &Value) -> bool {
        let value = &try_parse_size_bytes(value);
        let other = &try_parse_size_bytes(other);

        fn parse_value_as_f64(value: &Value) -> Option<f64> {
            match value {
                Value::Number(num) => num.as_f64(),
                Value::String(s) => s.parse::<f64>().ok(),
                _ => None,
            }
        }

        // Helper function to normalize values as lowercase strings
        fn normalize_string(value: &Value) -> Option<String> {
            match value {
                Value::String(s) => Some(s.to_lowercase()),
                Value::Number(num) => Some(num.to_string()),
                _ => None,
            }
        }

        // Extract numerical and string representations
        let value_num = parse_value_as_f64(value);
        let other_num = parse_value_as_f64(other);
        let value_str = normalize_string(value);
        let other_str = normalize_string(other);

        match self {
            CompareOp::Equal => value_num
                .zip(other_num)
                .map_or_else(|| value_str == other_str, |(a, b)| a == b),
            CompareOp::NotEqual => value_num
                .zip(other_num)
                .map_or_else(|| value_str != other_str, |(a, b)| a != b),
            CompareOp::Like | CompareOp::NotLike => {
                let similarity_threshold = 0.9;
                let is_like = value_num.zip(other_num).map_or_else(
                    || {
                        let value_str = value_str.unwrap_or_default();
                        let other_str = other_str.unwrap_or_default();
                        jaro_winkler(&value_str, &other_str) > similarity_threshold
                    },
                    |(a, b)| (a - b).abs() < b * 0.1,
                );
                matches!(self, CompareOp::Like) == is_like
            }
            CompareOp::GreaterThan => value_num
                .zip(other_num)
                .map_or_else(|| value_str > other_str, |(a, b)| a > b),
            CompareOp::LessThan => value_num
                .zip(other_num)
                .map_or_else(|| value_str < other_str, |(a, b)| a < b),
            CompareOp::GreaterThanOrEqual => value_num
                .zip(other_num)
                .map_or_else(|| value_str >= other_str, |(a, b)| a >= b),
            CompareOp::LessThanOrEqual => value_num
                .zip(other_num)
                .map_or_else(|| value_str <= other_str, |(a, b)| a <= b),
            CompareOp::Regex => {
                if let (Some(value_str), Some(other_str)) = (value_str, other_str) {
                    Regex::new(&other_str).is_ok_and(|re| re.is_match(&value_str))
                } else {
                    false
                }
            }
            CompareOp::Contains => {
                if let (Some(value_str), Some(other_str)) = (value_str, other_str) {
                    value_str.contains(&other_str)
                } else {
                    false
                }
            }
            CompareOp::StartsWith => {
                if let (Some(value_str), Some(other_str)) = (value_str, other_str) {
                    value_str.starts_with(&other_str)
                } else {
                    false
                }
            }
            CompareOp::EndsWith => {
                if let (Some(value_str), Some(other_str)) = (value_str, other_str) {
                    value_str.ends_with(&other_str)
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

pub fn value_matches_with_parents(
    json_value: &Value,
    remember_parent_key: &str,
    search_str: &str,
) -> Vec<String> {
    let search = match Search::from_str(search_str) {
        Ok(search) => search,
        Err(e) => {
            eprintln!("Failed to parse search string: {}", e);
            return vec![];
        }
    };

    let mut results = Vec::new();
    let mut found_match = false;

    recursive_collect(
        json_value,
        remember_parent_key,
        &search,
        None,
        &mut results,
        &mut found_match,
    );

    if results.is_empty() {
        if found_match {
            return vec!["".to_string()];
        } else {
            return vec![];
        }
    } else {
        // Clean up results, remove duplicates and empty strings
        results.sort_unstable();
        results.dedup();
        if results.len() > 1 {
            results.retain(|s| !s.is_empty());
        }
    }

    results
}

/// Recursively collect matches:
/// - Directly attempt to extract `remember_parent_key` at this node (no subtree search).
/// - If found, update `current_parent`.
/// - Check for matches. If matches and `current_parent` is Some&non-empty, record it.
///   Else if matches and no parent, record `""` only if not done before.
/// - Recurse into children.
fn recursive_collect(
    json_value: &Value,
    remember_parent_key: &str,
    search: &Search,
    last_seen_parent: Option<String>,
    results: &mut Vec<String>,
    found_match: &mut bool,
) {
    // Start with the parent's known value
    let mut current_parent = last_seen_parent;

    // If this node can directly resolve the `remember_parent_key`, update current_parent
    if let Some(val) = find_parent_key_direct(json_value, remember_parent_key) {
        current_parent = Some(val);
    }

    // Check for a match at this node
    if search.matches(json_value) {
        *found_match = true;
        match &current_parent {
            Some(p) if !p.is_empty() => {
                // Non-empty parent found
                results.push(p.clone());
            }
            _ => {
                // No parent or empty parent
                results.push("".to_string());
            }
        }
    }

    // Recurse into children
    match json_value {
        Value::Object(map) => {
            for v in map.values() {
                recursive_collect(
                    v,
                    remember_parent_key,
                    search,
                    current_parent.clone(),
                    results,
                    found_match,
                );
            }
        }
        Value::Array(arr) => {
            for v in arr {
                recursive_collect(
                    v,
                    remember_parent_key,
                    search,
                    current_parent.clone(),
                    results,
                    found_match,
                );
            }
        }
        _ => {}
    }
}

/// Directly find `remember_parent_key` from the current node.
/// This allows dot notation but does not search beyond the current object's direct structure.
fn find_parent_key_direct(obj: &Value, remember_parent_key: &str) -> Option<String> {
    let parts: Vec<&str> = remember_parent_key.split('.').collect();
    follow_path_parts(obj, &parts)
}

fn follow_path_parts(value: &Value, parts: &[&str]) -> Option<String> {
    if parts.is_empty() {
        // No more parts left, return string value if any
        return value.as_str().map(String::from);
    }

    match value {
        Value::Object(map) => {
            // Follow the next part in the object
            let part = parts[0];
            map.get(part)
                .and_then(|next| follow_path_parts(next, &parts[1..]))
        }
        Value::Array(arr) => {
            // Try to find a match in any of the array elements
            for element in arr {
                if let Some(val) = follow_path_parts(element, parts) {
                    return Some(val);
                }
            }
            None
        }
        _ => None,
    }
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
    fn test_comparison_operator_with_decimal() {
        let json_str = r#"
            {
                "stats": {
                    "price": "10.5"
                }
            }
        "#;
        let json_value: Value = from_str(json_str).unwrap();
        let search_str = "stats.price > 9.5";
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
        let search = Search::from_str("name = deep_value AND otherkey = other_value").unwrap();
        assert!(search.matches(&json_value));
        let search = Search::from_str("name = deep_value Or otherkey = other_value").unwrap();
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

    #[test]
    fn test_remember_parent_key() {
        let json_str = r#"
            {
                "defaults":{
                    "machine_spec": {"instance_types": [{ "id": "t2.micro", "memory": "512 MB" }]}
                },
                "regions":{
                    "instance_types": { "id": "t2.large" },
                    "memory": "1024 MB"
                },
                "something_else": {
                    "instance_types": { "id": "t2.nano" },
                    "memory": "512 MB"
                }
            }
        "#;
        let json_value: Value = serde_json::from_str(json_str).unwrap();
        let remember_parent_key = "instance_types.id";
        let search_str = "memory = 512 MB";

        let results = value_matches_with_parents(&json_value, remember_parent_key, search_str);

        assert_eq!(results, vec!["t2.micro", "t2.nano"]);
    }

    #[test]
    fn test_remember_parent_key_complex() {
        let json_str = r#"{"api_version":"v0.1.0","defaults":{"backup":null,"compliance":null,"cost_optimization":null,"machine_spec":{"instance_types":[{"ai_spec":null,"compliance":null,"cpu":"0.5 vCPUs","description":null,"gpu":null,"id":"xxx-small","memory":"512 MB","metadata":{"availability":"medium","optimized_for":"general"},"network":null,"pricing":{"on_demand":{"hour":"0.01"},"reserved":{"three_year":"20","year":"10"}},"storage":{"iops":null,"size":"2 GB","type":"SSD"},"tags":null,"type":"general-purpose"}]},"monitoring":{"enabled":true,"logging":{"enabled":true,"log_retention":"30 days"},"metrics":{"cpu_utilization":true,"disk_iops":true,"memory_usage":true,"network_traffic":true}},"network_spec":{"firewalls":null,"load_balancers":{"type":["network"]},"private_ip":true,"public_ip":true,"vpc_support":true},"security":null,"service_integrations":null,"sla":null},"kind":"Offering","metadata":{"name":"Demo Node Provider, do not use","version":"1.0"},"provider":{"description":"a generic offering specification for a cloud provider","name":"generic cloud provider"},"regions":[{"availability_zones":[{"description":"primary availability zone","name":"eu-central-1a"},{"description":"secondary availability zone","name":"eu-central-1b"}],"compliance":["GDPR"],"description":"central europe region","geography":{"continent":"Europe","country":"Germany","iso_codes":{"country_code":"DE","region_code":"EU"}},"machine_spec":null,"name":"eu-central-1"},{"availability_zones":[{"description":"primary availability zone","name":"us-east-1a"},{"description":"secondary availability zone","name":"us-east-1b"}],"compliance":["SOC 2"],"description":"united states east coast region","geography":{"continent":"North America","country":"United States","iso_codes":{"country_code":"US","region_code":"NA"}},"machine_spec":null,"name":"us-east-1"}]}
        "#;
        let json_value: Value = serde_json::from_str(json_str).unwrap();
        let remember_parent_key = "instance_types.id";
        let search_str = "memory >= 512 MB";

        let results = value_matches_with_parents(&json_value, remember_parent_key, search_str);

        assert_eq!(results, vec!["xxx-small"]);
    }

    #[test]
    fn test_no_parent_key_found() {
        let json_str = r#"
            [
                {
                    "instance_types": { "id": "t2.micro" },
                    "memory": "512 MB"
                },
                {
                    "instance_types": { "id": "t2.large" },
                    "memory": "1024 MB"
                }
            ]
        "#;
        let json_value: Value = serde_json::from_str(json_str).unwrap();
        let remember_parent_key = "nonexistent.key";
        let search_str = "memory = 512 MB";

        let results = value_matches_with_parents(&json_value, remember_parent_key, search_str);

        assert_eq!(results, vec![""]);
    }
}
