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
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::InvalidUsername(u) => write!(f, "Invalid username: {u}"),
            AuthError::InvalidPassword(u) => write!(f, "Invalid password for user: {u}"),
            AuthError::UserNotFound(u) => write!(f, "User not found: {u}"),
            AuthError::MalformedInput(s) => write!(f, "Malformed input: {s}"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Storage module errors
#[derive(Debug)]
pub enum StorageError {
    FileNotFound(String),
    DirectoryNotFound(String),
    InvalidPath(String),
    FileAlreadyExists(String),
    NotADirectory(String),
    PermissionDenied(String),
    IoError(io::Error),
    UploadInProgress(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::FileNotFound(p) => write!(f, "File not found: {p}"),
            StorageError::DirectoryNotFound(p) => write!(f, "Directory not found: {p}"),
            StorageError::InvalidPath(p) => write!(f, "Invalid path: {p}"),
            StorageError::FileAlreadyExists(p) => write!(f, "File already exists: {p}"),
            StorageError::NotADirectory(p) => write!(f, "Not a directory: {p}"),
            StorageError::PermissionDenied(p) => write!(f, "Permission denied: {p}"),
            StorageError::IoError(e) => write!(f, "IO error: {e}"),
            StorageError::UploadInProgress(p) => write!(f, "Upload already in progress: {p}"),
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
    PortBindingFailed(SocketAddr, io::Error),
    NoAvailablePort,
    ListenerConfigurationFailed(io::Error),
    DataChannelSetupFailed(String),
    InvalidPortCommand(String),
    IpMismatch { expected: String, provided: String },
    InvalidPortRange(u16),
    TransferFailed(io::Error),
}

impl fmt::Display for TransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransferError::PortBindingFailed(addr, e) => {
                write!(f, "Failed to bind to {addr}: {e}")
            }
            TransferError::NoAvailablePort => write!(f, "No available port for data connection"),
            TransferError::ListenerConfigurationFailed(e) => {
                write!(f, "Failed to configure listener: {e}")
            }
            TransferError::DataChannelSetupFailed(msg) => {
                write!(f, "Data channel setup failed: {msg}")
            }
            TransferError::InvalidPortCommand(msg) => write!(f, "Invalid PORT command: {msg}"),
            TransferError::IpMismatch { expected, provided } => {
                write!(f, "IP mismatch: expected {expected}, got {provided}")
            }
            TransferError::InvalidPortRange(port) => {
                write!(f, "Invalid port {port}: must be between 1024 and 65535")
            }
            TransferError::TransferFailed(e) => write!(f, "Transfer failed: {e}"),
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
            NavigateError::InvalidPath(p) => write!(f, "Invalid path: {p}"),
            NavigateError::DirectoryNotFound(p) => write!(f, "Directory not found: {p}"),
            NavigateError::NotADirectory(p) => write!(f, "Not a directory: {p}"),
            NavigateError::PermissionDenied(p) => write!(f, "Permission denied: {p}"),
            NavigateError::PathTraversal(p) => write!(f, "Path traversal attempt: {p}"),
        }
    }
}

impl std::error::Error for NavigateError {}
