//! Error types
//!
//! Defines domain-specific error types for each module of the FTP server.

use std::fmt;
use std::io;
use std::net::SocketAddr;

/// Authentication module errors
#[derive(Debug)]
pub enum AuthError {
    InvalidUsername(String),
    InvalidPassword(String),
    UserNotFound(String),
    MalformedInput(String),
    NotLoggedIn,
    InvalidState(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::InvalidUsername(u) => write!(f, "Invalid username: {}", u),
            AuthError::InvalidPassword(u) => write!(f, "Invalid password for user: {}", u),
            AuthError::UserNotFound(u) => write!(f, "User not found: {}", u),
            AuthError::MalformedInput(s) => write!(f, "Malformed input: {}", s),
            AuthError::NotLoggedIn => write!(f, "User not logged in"),
            AuthError::InvalidState(s) => write!(f, "Invalid state: {}", s),
        }
    }
}

impl std::error::Error for AuthError {}

/// Storage module errors
#[derive(Debug)]
pub enum StorageError {
    FileNotFound(String),
    DirectoryNotFound(String),
    PermissionDenied(String),
    InvalidPath(String),
    FileAlreadyExists(String),
    NotADirectory(String),
    IoError(io::Error),
    PathTraversal(String),
    UploadInProgress(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::FileNotFound(p) => write!(f, "File not found: {}", p),
            StorageError::DirectoryNotFound(p) => write!(f, "Directory not found: {}", p),
            StorageError::PermissionDenied(p) => write!(f, "Permission denied: {}", p),
            StorageError::InvalidPath(p) => write!(f, "Invalid path: {}", p),
            StorageError::FileAlreadyExists(p) => write!(f, "File already exists: {}", p),
            StorageError::NotADirectory(p) => write!(f, "Not a directory: {}", p),
            StorageError::IoError(e) => write!(f, "IO error: {}", e),
            StorageError::PathTraversal(p) => write!(f, "Path traversal attempt: {}", p),
            StorageError::UploadInProgress(p) => write!(f, "Upload already in progress: {}", p),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<io::Error> for StorageError {
    fn from(error: io::Error) -> Self {
        StorageError::IoError(error)
    }
}

/// Transfer module errors
#[derive(Debug)]
pub enum TransferError {
    DataChannelNotInitialized,
    PortBindingFailed(SocketAddr, io::Error),
    NoAvailablePort,
    ListenerConfigurationFailed(io::Error),
    ConnectionTimeout(SocketAddr),
    DataChannelSetupFailed(String),
    InvalidPortCommand(String),
    IpMismatch { expected: String, provided: String },
    InvalidPortRange(u16),
    TransferFailed(io::Error),
}

impl fmt::Display for TransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransferError::DataChannelNotInitialized => write!(f, "Data channel not initialized"),
            TransferError::PortBindingFailed(addr, e) => {
                write!(f, "Failed to bind to {}: {}", addr, e)
            }
            TransferError::NoAvailablePort => write!(f, "No available port for data connection"),
            TransferError::ListenerConfigurationFailed(e) => {
                write!(f, "Failed to configure listener: {}", e)
            }
            TransferError::ConnectionTimeout(addr) => {
                write!(f, "Timeout waiting for connection from {}", addr)
            }
            TransferError::DataChannelSetupFailed(msg) => {
                write!(f, "Data channel setup failed: {}", msg)
            }
            TransferError::InvalidPortCommand(msg) => write!(f, "Invalid PORT command: {}", msg),
            TransferError::IpMismatch { expected, provided } => {
                write!(f, "IP mismatch: expected {}, got {}", expected, provided)
            }
            TransferError::InvalidPortRange(port) => {
                write!(f, "Invalid port {}: must be between 1024 and 65535", port)
            }
            TransferError::TransferFailed(e) => write!(f, "Transfer failed: {}", e),
        }
    }
}

impl std::error::Error for TransferError {}

/// Navigate module errors
#[derive(Debug)]
pub enum NavigateError {
    InvalidPath(String),
    DirectoryNotFound(String),
    NotADirectory(String),
    PermissionDenied(String),
    PathTraversal(String),
}

impl fmt::Display for NavigateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NavigateError::InvalidPath(p) => write!(f, "Invalid path: {}", p),
            NavigateError::DirectoryNotFound(p) => write!(f, "Directory not found: {}", p),
            NavigateError::NotADirectory(p) => write!(f, "Not a directory: {}", p),
            NavigateError::PermissionDenied(p) => write!(f, "Permission denied: {}", p),
            NavigateError::PathTraversal(p) => write!(f, "Path traversal attempt: {}", p),
        }
    }
}

impl std::error::Error for NavigateError {}

/// Client module errors
#[derive(Debug)]
pub enum ClientError {
    ClientNotFound(SocketAddr),
    InvalidState(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::ClientNotFound(addr) => write!(f, "Client not found: {}", addr),
            ClientError::InvalidState(msg) => write!(f, "Invalid client state: {}", msg),
        }
    }
}

impl std::error::Error for ClientError {}

/// General FTP server error that encompasses all error types
#[derive(Debug)]
pub enum FtpServerError {
    Auth(AuthError),
    Storage(StorageError),
    Transfer(TransferError),
    Navigate(NavigateError),
    Client(ClientError),
    IoError(io::Error),
    NetworkError(String),
    ProtocolError(String),
    FileSystemError(String),
}

impl fmt::Display for FtpServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FtpServerError::Auth(e) => write!(f, "Authentication error: {}", e),
            FtpServerError::Storage(e) => write!(f, "Storage error: {}", e),
            FtpServerError::Transfer(e) => write!(f, "Transfer error: {}", e),
            FtpServerError::Navigate(e) => write!(f, "Navigate error: {}", e),
            FtpServerError::Client(e) => write!(f, "Client error: {}", e),
            FtpServerError::IoError(e) => write!(f, "I/O error: {}", e),
            FtpServerError::NetworkError(e) => write!(f, "Network error: {}", e),
            FtpServerError::ProtocolError(e) => write!(f, "Protocol error: {}", e),
            FtpServerError::FileSystemError(e) => write!(f, "File system error: {}", e),
        }
    }
}

impl std::error::Error for FtpServerError {}

// Implement conversions from specific errors to FtpServerError
impl From<AuthError> for FtpServerError {
    fn from(error: AuthError) -> Self {
        FtpServerError::Auth(error)
    }
}

impl From<StorageError> for FtpServerError {
    fn from(error: StorageError) -> Self {
        FtpServerError::Storage(error)
    }
}

impl From<TransferError> for FtpServerError {
    fn from(error: TransferError) -> Self {
        FtpServerError::Transfer(error)
    }
}

impl From<NavigateError> for FtpServerError {
    fn from(error: NavigateError) -> Self {
        FtpServerError::Navigate(error)
    }
}

impl From<ClientError> for FtpServerError {
    fn from(error: ClientError) -> Self {
        FtpServerError::Client(error)
    }
}

impl From<io::Error> for FtpServerError {
    fn from(error: io::Error) -> Self {
        FtpServerError::IoError(error)
    }
}
