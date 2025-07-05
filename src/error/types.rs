//! Error types
//! 
//! Defines custom error types for the FTP server.

use std::fmt;

/// Main error type for the FTP server
#[derive(Debug)]
pub enum FtpServerError {
    IoError(std::io::Error),
    NetworkError(String),
    AuthenticationError(String),
    ProtocolError(String),
    FileSystemError(String),
}

impl fmt::Display for FtpServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FtpServerError::IoError(e) => write!(f, "IO Error: {}", e),
            FtpServerError::NetworkError(e) => write!(f, "Network Error: {}", e),
            FtpServerError::AuthenticationError(e) => write!(f, "Authentication Error: {}", e),
            FtpServerError::ProtocolError(e) => write!(f, "Protocol Error: {}", e),
            FtpServerError::FileSystemError(e) => write!(f, "File System Error: {}", e),
        }
    }
}

impl std::error::Error for FtpServerError {}

impl From<std::io::Error> for FtpServerError {
    fn from(error: std::io::Error) -> Self {
        FtpServerError::IoError(error)
    }
}
