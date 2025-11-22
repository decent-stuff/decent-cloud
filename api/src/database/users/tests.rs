use super::*;

#[test]
fn export_typescript_types() {
    UserActivity::export().expect("Failed to export UserActivity type");
    AccountContact::export().expect("Failed to export AccountContact type");
    AccountSocial::export().expect("Failed to export AccountSocial type");
    AccountExternalKey::export().expect("Failed to export AccountExternalKey type");
}
