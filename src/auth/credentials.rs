//! Credential storage and management
//!
//! Handles user credential storage and validation.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Static credential store - in production this would be a proper database
static CREDENTIALS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut creds = HashMap::new();
    creds.insert("alice", "alice123");
    creds.insert("bob", "bob123");
    creds.insert("admin", "admin123");
    creds
});

/// Check if username exists
pub fn user_exists(username: &str) -> bool {
    CREDENTIALS.contains_key(username)
}

/// Validate username and password combination
pub fn validate_credentials(username: &str, password: &str) -> bool {
    match CREDENTIALS.get(username) {
        Some(stored_password) => stored_password == &password,
        None => false,
    }
}
