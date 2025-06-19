use std::collections::HashMap;
use std::sync::LazyLock;

// Custom error type for authentication
#[derive(Debug)]
pub struct AuthError {
    message: &'static str,
    ftp_code: &'static str,
}

impl AuthError {
    pub fn new(message: &'static str, ftp_code: &'static str) -> Self {
        AuthError { message, ftp_code }
    }

    pub fn ftp_response(&self) -> &'static str {
        self.ftp_code
    }

    pub fn message(&self) -> &'static str {
        self.message
    }
}

// Predefined usernames and passwords
static CREDENTIALS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut cred_db = HashMap::new();
    cred_db.insert("alice", "alice123");
    cred_db.insert("bob", "bob123");
    cred_db.insert("admin", "admin123");
    cred_db
});

// Validate username
pub fn validate_user(username: &str) -> Result<&'static str, AuthError> {
    if CREDENTIALS.contains_key(username) {
        Ok("331 Password required\r\n")
    } else {
        Err(AuthError::new("Invalid username", "530"))
    }
}

// Validate password for a given username
pub fn validate_password(username: &str, password: &str) -> Result<&'static str, AuthError> {
    match CREDENTIALS.get(username) {
        Some(stored_password) if stored_password == &password => Ok("230 Login successful\r\n"),
        Some(_) => Err(AuthError::new("Invalid password", "530")),
        None => Err(AuthError::new("Invalid username", "530")),
    }
}
