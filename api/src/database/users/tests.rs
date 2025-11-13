use super::*;

#[test]
fn export_typescript_types() {
    UserProfile::export().expect("Failed to export UserProfile type");
    UserContact::export().expect("Failed to export UserContact type");
    UserSocial::export().expect("Failed to export UserSocial type");
    UserPublicKey::export().expect("Failed to export UserPublicKey type");
}
