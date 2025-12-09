pub mod email_service;
pub mod validation;

pub use email_service::{EmailAttachment, EmailService};
pub use validation::validate_email;
