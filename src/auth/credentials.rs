//! Credential storage and management
//!
//! Handles user credential storage and validation.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Static credential store - in production this would be a proper database
pub(crate) static CREDENTIALS: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        let mut creds = HashMap::new();
        creds.insert("alice", "alice123");
        creds.insert("bob", "bob123");
        creds.insert("admin", "admin123");
        creds
    });
