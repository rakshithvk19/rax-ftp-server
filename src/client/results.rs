//! Client result types
//!
//! Defines result structures returned by client operations.

/// Result of a logout operation
#[derive(Debug, Clone)]
pub struct LogoutResult {
    pub was_logged_in: bool,
}

/// Result of a quit operation
#[derive(Debug, Clone)]
pub struct QuitResult {
    pub client_addr: String,
}
