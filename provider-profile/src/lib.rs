use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml_ng::{self, Value as YamlValue};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum Profile {
    V0_1_0(ProfileV0_1_0),
    // Future versions can be added as new variants
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct ProfileV0_1_0 {
    pub kind: String,
    pub metadata: Metadata,
    pub api_version: String,
    pub spec: Spec,

    // Add a field to hold the raw JsonValue representation, for use in matches_search
    #[serde(skip)]
    #[borsh(skip)]
    _json_value: JsonValue,
}

impl ProfileV0_1_0 {
    pub fn validate(&self) -> Result<(), String> {
        if self.kind.as_str() != "Profile" {
            return Err(format!("Unsupported kind '{}'", self.kind));
        }
        Ok(())
    }

    pub fn matches_search(&self, _search_str: &str) -> bool {
        false
    }
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Metadata {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Spec {
    pub description: Option<String>,
    pub url: Option<String>,
    pub logo_url: Option<String>,
    pub why_choose_us: Option<String>,
    pub contacts: HashMap<String, String>,
}

impl Profile {
    pub fn new_from_file(path: &str) -> Result<Self, String> {
        let input =
            fs_err::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
        if path.to_lowercase().ends_with(".json") {
            Self::new_from_str(&input, "json")
        } else if path.to_lowercase().ends_with(".yaml") {
            Self::new_from_str(&input, "yaml")
        } else {
            Err("Unsupported file format. Use '.json' or '.yaml'.".to_string())
        }
    }

    pub fn new_from_str(input: &str, format: &str) -> Result<Self, String> {
        let doc: JsonValue = match format {
            "yaml" => {
                let yaml_value: YamlValue = serde_yaml_ng::from_str(input)
                    .map_err(|e| format!("Failed to parse YAML: {}", e))?;
                serde_json::to_value(yaml_value)
                    .map_err(|e| format!("Failed to convert YAML to JSON value: {}", e))?
            }
            "json" => {
                serde_json::from_str(input).map_err(|e| format!("Failed to parse JSON: {}", e))?
            }
            _ => return Err("Unsupported format. Use 'yaml' or 'json'.".to_string()),
        };

        match doc.get("api_version").and_then(|v| v.as_str()) {
            Some("v0.1.0") => serde_json::from_value::<ProfileV0_1_0>(doc.clone())
                .map(Profile::V0_1_0)
                .map(|v| v.compute_json_value())
                .map_err(|e| {
                    format!(
                        "Failed to deserialize Profile due to err {} from input doc {}",
                        e, doc
                    )
                }),
            Some(version) => Err(format!("Unsupported api_version '{}'", version)),
            None => Err("Missing 'api_version' field.".to_string()),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        match self {
            Profile::V0_1_0(profile) => profile.validate(),
        }
    }

    pub fn matches_search(&self, search_str: &str) -> bool {
        match self {
            Profile::V0_1_0(profile) => profile.matches_search(search_str),
        }
    }

    pub fn compute_json_value(self) -> Self {
        match self {
            Profile::V0_1_0(profile) => Profile::V0_1_0(ProfileV0_1_0 {
                _json_value: serde_json::to_value(&profile).unwrap(),
                ..profile
            }),
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Profile::V0_1_0(profile) => write!(f, "{}", profile),
        }
    }
}

impl fmt::Display for ProfileV0_1_0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_yaml_ng::to_string(self) {
            Ok(yaml_str) => write!(f, "{}", yaml_str),
            Err(e) => {
                write!(f, "Failed to format ProfileV0_1_0: {}", e)?;
                Err(fmt::Error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_search_yaml() {
        let profile_yaml = r#"
            api_version: v0.1.0
            kind: Profile
            metadata:
                name: "Test Node Provider"
                version: "0.0.1"
            spec:
                description: "Just a test"
                url: "https://example.com"
                logo_url: "https://example.com/logo.jpg"
                why_choose_us: "Because we're the best!"
                contacts:
                    Twitter: "x.com/dc-prov"
                    Linkedin: "linkedin.com/dc-prov"
                    email: "support@dc-prov.com"
        "#;

        let profile = Profile::new_from_str(profile_yaml, "yaml").expect("Failed to parse YAML");

        match profile {
            Profile::V0_1_0(ref p) => {
                assert_eq!(p.metadata.name, "Test Node Provider");
                assert_eq!(p.kind, "Profile");
            }
        }
        assert!(profile.matches_search("name=Test Node Provider"));
        assert!(profile.matches_search("Twitter contains x.com/dc-prov"));
    }
}
