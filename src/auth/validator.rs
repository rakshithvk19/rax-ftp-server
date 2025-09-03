//! Authentication validator
//!
//! Implements FTP user authentication logic, including username and password validation.
//! Uses a static in-memory credential store for demonstration purposes.

use super::credentials::CREDENTIALS;
use crate::error::AuthError;

/// Performs basic input sanitation to check for malicious or malformed usernames/passwords.
fn is_valid_input(input: &str) -> bool {
    !input.trim().is_empty() && input.len() <= 64 && !input.contains(['\r', '\n', '\0'])
}

/// Validates that the given username exists in the credential store.
pub fn validate_user(username: &str) -> Result<(), AuthError> {
    // Check for invalid username characters/format
    if username.contains(['@', '#', '$', '%']) || username.starts_with(char::is_numeric) {
        return Err(AuthError::InvalidUsername(username.to_string()));
    }

    if !is_valid_input(username) {
        return Err(AuthError::MalformedInput("Invalid username format".into()));
    }

    if CREDENTIALS.contains_key(username) {
        Ok(())
    } else {
        Err(AuthError::UserNotFound(username.to_string()))
    }
}

/// Validates that the provided password matches the stored password for the username.
pub fn validate_password(username: &str, password: &str) -> Result<(), AuthError> {
    if !is_valid_input(password) {
        return Err(AuthError::MalformedInput("Invalid password format".into()));
    }

    match CREDENTIALS.get(username) {
        Some(stored) if stored == &password => Ok(()),
        Some(_) => Err(AuthError::InvalidPassword(username.to_string())),
        None => Err(AuthError::UserNotFound(username.to_string())),
    }
}
