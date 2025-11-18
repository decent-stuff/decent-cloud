use super::*;
use regex::Regex;
use std::collections::HashSet;

const MAIN_RS_SOURCE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"));

fn collect_route_paths(source: &str) -> Vec<String> {
    let regex = Regex::new(r#"\.at\(\s*"([^"]+)""#).expect("invalid route regex");
    regex
        .captures_iter(source)
        .map(|captures| captures[1].to_string())
        .collect()
}

fn duplicate_paths(paths: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();
    for path in paths {
        let inserted = seen.insert(path.clone());
        if !inserted && !duplicates.contains(path) {
            duplicates.push(path.clone());
        }
    }
    duplicates
}

#[test]
fn router_configuration_has_unique_paths() {
    let paths = collect_route_paths(MAIN_RS_SOURCE);
    let duplicates = duplicate_paths(&paths);
    assert!(
        duplicates.is_empty(),
        "Duplicate route paths detected: {:?}",
        duplicates
    );
}

#[test]
fn duplicate_detector_identifies_duplicates() {
    let sample = vec![
        "/first".to_string(),
        "/second".to_string(),
        "/first".to_string(),
        "/second".to_string(),
    ];
    let duplicates = duplicate_paths(&sample);
    assert_eq!(
        duplicates,
        vec!["/first".to_string(), "/second".to_string()]
    );
}

#[test]
fn root_route_configured() {
    let paths = collect_route_paths(MAIN_RS_SOURCE);
    assert!(
        paths.contains(&"/".to_string()),
        "Root path '/' should be configured in routes"
    );
}

#[test]
fn root_redirect_targets_swagger() {
    assert!(
        MAIN_RS_SOURCE.contains("Redirect::temporary(\"/api/v1/swagger\")"),
        "Root redirect should target /api/v1/swagger"
    );
}
