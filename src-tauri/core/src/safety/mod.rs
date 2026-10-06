pub mod protected;
pub mod validator;

pub use protected::{classify_protection, zones, Guard, ProtectedZone};
pub use validator::{validate_targets, validate_user_targets, ValidationContext};
