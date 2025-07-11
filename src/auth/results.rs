//! Authentication result types
//!
//! Defines result structures returned by authentication operations.

/// Result of user validation
#[derive(Debug, Clone)]
pub struct UserValidationResult {
    pub user_valid: bool,
    pub username: String,
}

/// Result of password validation
#[derive(Debug, Clone)]
pub struct PasswordValidationResult {
    pub login_successful: bool,
    pub username: String,
}
