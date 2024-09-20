// profile_parser.rs

use std::collections::HashMap;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Debug)]
pub enum Profile {
    V0_1_0(ProfileV0_1_0),
    // Add future versions here as new variants, e.g., V0_2_0(ProfileV0_2_0)
}

#[derive(Debug)]
pub struct ProfileV0_1_0 {
    pub kind: String,
    pub metadata: Metadata,
    pub spec: Spec,
}

#[derive(Debug)]
pub struct Metadata {
    pub name: String,
    pub version: String,
}

#[derive(Debug)]
pub struct Spec {
    pub description: String,
    pub url: String,
    pub logo_url: String,
    pub why_choose_us: String,
    pub contacts: HashMap<String, String>,
}

impl Profile {
    // Function to parse the YAML string into a Profile enum, specific to the apiVersion v0.1.0
    pub fn parse(yaml: &str) -> Result<Self, String> {
        let docs = YamlLoader::load_from_str(yaml).map_err(|_| "Failed to load YAML")?;
        let doc = &docs[0];

        let kind = doc["kind"].as_str().unwrap_or("");
        if kind != "Profile" {
            return Err(format!("Unsupported kind {}", kind));
        }
        // Check the apiVersion to determine which Profile variant to use
        let api_version = doc["apiVersion"].as_str().unwrap_or("");
        match api_version {
            "v0.1.0" => Self::parse_profile_v0_1_0(doc),
            // Future versions can be added here with additional cases
            _ => Err("Unsupported apiVersion".to_string()),
        }
    }

    // Function to search for a particular field by key and value
    pub fn search(&self, key: &str, value: &str) -> bool {
        match self {
            Profile::V0_1_0(profile) => profile.search(key, value),
            // Add future version search methods as needed
        }
    }
    fn parse_profile_v0_1_0(doc: &Yaml) -> Result<Self, String> {
        let kind = doc["kind"].as_str().unwrap_or("").to_string();
        let metadata = Metadata {
            name: doc["metadata"]["name"].as_str().unwrap_or("").to_string(),
            version: doc["metadata"]["version"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        };

        let spec = Spec {
            description: doc["spec"]["description"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            url: doc["spec"]["url"].as_str().unwrap_or("").to_string(),
            logo_url: doc["spec"]["logo_url"].as_str().unwrap_or("").to_string(),
            why_choose_us: doc["spec"]["why_choose_us"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            contacts: doc["spec"]["contacts"]
                .as_hash()
                .map(|hash| {
                    hash.iter()
                        .map(|(k, v)| {
                            (
                                k.as_str().unwrap_or("").to_string(),
                                v.as_str().unwrap_or("").to_string(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_else(HashMap::new),
        };

        Ok(Self::V0_1_0(ProfileV0_1_0 {
            kind,
            metadata,
            spec,
        }))
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
            "url" => self.spec.url.contains(value),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_search() {
        let yaml = r#"
            apiVersion: v0.1.0
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
            } // No other versions should be present in this test case
        }
    }

    #[test]
    fn test_unsupported_api_version() {
        let yaml = r#"
            apiVersion: v0.0.5
            kind: Profile
        "#;
        assert!(Profile::parse(yaml).is_err());
    }

    #[test]
    fn test_unsupported_kind() {
        let yaml = r#"
            apiVersion: v0.1.0
            kind: Offering
        "#;
        assert!(Profile::parse(yaml).is_err());
    }
}
