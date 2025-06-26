//! Module `auth`
//!
//! Implements FTP user authentication logic, including username and password validation,
//! along with custom error handling that encapsulates FTP response codes and messages.
//!
//! Uses a static in-memory credential store for demonstration purposes.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Custom error type for authentication failures.
/// Holds an FTP response code and a descriptive message.
#[derive(Debug)]
pub struct AuthError {
    /// Descriptive error message for logging or diagnostics.
    message: &'static str,
    /// FTP status code string to be sent to client as response.
    ftp_code: &'static str,
}

impl AuthError {
    /// Creates a new `AuthError` with the given message and FTP code.
    ///
    /// # Arguments
    /// * `message` - A human-readable error message.
    /// * `ftp_code` - FTP protocol response code (e.g., "530").
    ///
    /// # Returns
    /// New instance of `AuthError`.
    pub fn new(message: &'static str, ftp_code: &'static str) -> Self {
        AuthError { message, ftp_code }
    }

    /// Returns the FTP protocol response code as a string slice.
    /// This code can be sent directly to the FTP client.
    pub fn ftp_response(&self) -> &'static str {
        self.ftp_code
    }

    /// Returns the descriptive error message.
    pub fn message(&self) -> &'static str {
        self.message
    }
}

/// Static in-memory credential store mapping usernames to passwords.
/// Uses `LazyLock` for safe one-time initialization on first access.
///
/// Note: For production, replace with a secure persistent store or authentication service.
static CREDENTIALS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut cred_db = HashMap::new();
    cred_db.insert("alice", "alice123");
    cred_db.insert("bob", "bob123");
    cred_db.insert("admin", "admin123");
    cred_db
});

/// Validates that the given username exists in the credential store.
///
/// # Arguments
/// * `username` - The username string provided by the FTP client.
///
/// # Returns
/// * `Ok` with FTP response string "331 Password required" if username is valid.
/// * `Err` with `AuthError` indicating invalid username if not found.
pub fn validate_user(username: &str) -> Result<&'static str, AuthError> {
    if CREDENTIALS.contains_key(username) {
        Ok("331 Password required\r\n")
    } else {
        Err(AuthError::new("Invalid username", "530"))
    }
}

/// Validates that the provided password matches the stored password for the username.
///
/// # Arguments
/// * `username` - Username string.
/// * `password` - Password string provided by the FTP client.
///
/// # Returns
/// * `Ok` with FTP response "230 Login successful" if credentials match.
/// * `Err` with `AuthError` indicating invalid password or username otherwise.
pub fn validate_password(username: &str, password: &str) -> Result<&'static str, AuthError> {
    match CREDENTIALS.get(username) {
        Some(stored_password) if stored_password == &password => Ok("230 Login successful\r\n"),
        Some(_) => Err(AuthError::new("Invalid password", "530")),
        None => Err(AuthError::new("Invalid username", "530")),
    }
}
