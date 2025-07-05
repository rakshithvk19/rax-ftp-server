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
    message: String,
    /// FTP status code string to be sent to client as response.
    ftp_code: &'static str,
}

impl AuthError {
    /// Creates a new `AuthError` with the given message and FTP code.
    pub fn new(message: impl Into<String>, ftp_code: &'static str) -> Self {
        AuthError {
            message: message.into(),
            ftp_code,
        }
    }

    /// Returns the FTP protocol response code as a string slice.
    pub fn ftp_response(&self) -> &'static str {
        self.ftp_code
    }

    /// Returns the descriptive error message for logging/debugging.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Static in-memory credential store mapping usernames to passwords.
/// Note: For production, replace with a secure persistent store or authentication service.
static CREDENTIALS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut cred_db = HashMap::new();
    cred_db.insert("alice", "alice123");
    cred_db.insert("bob", "bob123");
    cred_db.insert("admin", "admin123");
    cred_db
});

/// Performs basic input sanitation to check for malicious or malformed usernames/passwords.
fn is_valid_input(input: &str) -> bool {
    !input.trim().is_empty()
        && input.len() <= 64
        && !input.contains(|c: char| c == '\r' || c == '\n' || c == '\0')
}

/// Validates that the given username exists in the credential store.
///
/// # Returns
/// * `Ok("331 Password required\r\n")` if username is valid.
/// * `Err(AuthError)` if username is invalid or input is unsafe.
pub fn validate_user(username: &str) -> Result<&'static str, AuthError> {
    if !is_valid_input(username) {
        return Err(AuthError::new("Malformed username", "530"));
    }

    if CREDENTIALS.contains_key(username) {
        Ok("331 Password required\r\n")
    } else {
        Err(AuthError::new(
            format!("Unknown user '{}'", username),
            "530",
        ))
    }
}

/// Validates that the provided password matches the stored password for the username.
///
/// # Returns
/// * `Ok("230 Login successful\r\n")` if credentials match.
/// * `Err(AuthError)` if user doesn't exist, or password is incorrect/malformed.
pub fn validate_password(username: &str, password: &str) -> Result<&'static str, AuthError> {
    if !is_valid_input(password) {
        return Err(AuthError::new("Malformed password", "530"));
    }

    match CREDENTIALS.get(username) {
        Some(stored) if stored == &password => Ok("230 Login successful\r\n"),
        Some(_) => Err(AuthError::new(
            format!("Invalid password for '{}'", username),
            "530",
        )),
        None => Err(AuthError::new(
            format!("Attempt to login as unknown user '{}'", username),
            "530",
        )),
    }
}
