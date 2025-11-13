use super::*;

#[test]
fn export_typescript_types() {
    ProviderProfile::export().expect("Failed to export ProviderProfile type");
    Validator::export().expect("Failed to export Validator type");
}
