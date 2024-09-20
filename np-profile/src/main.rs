use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// Define the Profile enum with version-specific variants
#[derive(Debug, Serialize, Deserialize)]
pub enum Profile {
    V0_1_0(ProfileV0_1_0),
    // Future versions can be added as new variants
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileV0_1_0 {
    pub kind: String,
    pub metadata: Metadata,
    pub spec: Spec,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Spec {
    pub description: String,
    pub url: String,
    pub logo_url: String,
    pub why_choose_us: String,
    pub contacts: HashMap<String, String>,
}

impl Profile {
    // Function to parse the YAML string into a Profile enum, specific to the api_version v0.1.0
    pub fn parse(yaml: &str) -> Result<Self, String> {
        // Load the YAML and deserialize into a Profile struct based on the api_version
        let doc: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(yaml).map_err(|e| format!("Failed to parse YAML: {}", e))?;

        // Validate the kind
        let kind = doc["kind"]
            .as_str()
            .ok_or("Missing or invalid 'kind' field")?;
        if kind != "Profile" {
            return Err(format!("Unsupported kind '{}'", kind));
        }

        // Check the api_version to determine which Profile variant to use
        let api_version = doc["api_version"]
            .as_str()
            .ok_or("Missing or invalid 'api_version' field")?;
        match api_version {
            "v0.1.0" => Self::parse_profile_v0_1_0(doc),
            _ => Err(format!("Unsupported api_version '{}'", api_version)),
        }
    }

    fn parse_profile_v0_1_0(doc: serde_yaml_ng::Value) -> Result<Self, String> {
        serde_yaml_ng::from_value(doc)
            .map(Profile::V0_1_0)
            .map_err(|e| format!("Failed to deserialize ProfileV0_1_0: {}", e))
    }

    // Function to search for a particular field by key and value
    pub fn search(&self, key: &str, value: &str) -> bool {
        match self {
            Profile::V0_1_0(profile) => profile.search(key, value),
            // Add future version search methods as needed
        }
    }
}

impl ProfileV0_1_0 {
    // Function to search within the v0.1.0 profile fields
    pub fn search(&self, key: &str, value: &str) -> bool {
        match key {
            "kind" => self.kind == value,
            "name" => self.metadata.name.contains(value),
            "version" => self.metadata.version == value,
            "description" => self.spec.description.contains(value),
            "url" => self.spec.url == value,
            "logo_url" => self.spec.logo_url == value,
            "why_choose_us" => self.spec.why_choose_us.contains(value),
            _ => self
                .spec
                .contacts
                .get(key)
                .map_or(false, |v| v.contains(value)),
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Profile::V0_1_0(profile) => write!(f, "{}", profile),
            // Add future versions' display methods as needed
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <YAML file>", args[0]);
        std::process::exit(1);
    }
    let yaml = std::fs::read_to_string(&args[1]).expect("Failed to read YAML file");
    match Profile::parse(&yaml) {
        Ok(profile) => println!("{}", profile),
        Err(e) => {
            eprintln!("Error parsing profile: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_search() {
        let yaml = r#"
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

        let profile = Profile::parse(yaml).expect("Failed to parse YAML");

        match profile {
            Profile::V0_1_0(p) => {
                assert_eq!(p.metadata.name, "Test Node Provider");
                assert_eq!(p.kind, "Profile");
                assert!(p.search("name", "Test Node Provider"));
                assert!(p.search("Twitter", "x.com/dc-prov"));
            }
        }
    }

    #[test]
    fn test_unsupported_api_version() {
        let yaml = r#"
            api_version: v0.0.5
            kind: Profile
        "#;
        assert!(Profile::parse(yaml).is_err());
    }

    #[test]
    fn test_unsupported_kind() {
        let yaml = r#"
            api_version: v0.1.0
            kind: Offering
        "#;
        assert!(Profile::parse(yaml).is_err());
    }
}
