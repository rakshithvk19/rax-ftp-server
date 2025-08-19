//! Protocol translators
//!
//! Translates domain-specific results and errors to FTP protocol responses.

use crate::auth::{PasswordValidationResult, UserValidationResult};
use crate::error::{AuthError, ClientError, NavigateError, StorageError, TransferError};
use crate::navigate::{CwdResult, PwdResult};
use crate::protocol::{CommandResult, CommandStatus};
use crate::storage::{DeleteResult, ListResult, RetrieveResult, StoreResult};
use crate::transfer::{ActiveModeResult, PassiveModeResult};

/// Translates authentication errors to FTP responses
pub fn auth_error_to_ftp_response(error: AuthError) -> CommandResult {
    let (code, message) = match error {
        AuthError::InvalidUsername(u) => (530, format!("Invalid username: {}", u)),
        AuthError::InvalidPassword(u) => (530, format!("Invalid password for user: {}", u)),
        AuthError::UserNotFound(u) => (530, format!("Unknown user '{}'", u)),
        AuthError::MalformedInput(_) => (530, "Malformed input".to_string()),
        AuthError::NotLoggedIn => (530, "Not logged in".to_string()),
        AuthError::InvalidState(s) => (530, s),
    };

    CommandResult {
        status: CommandStatus::Failure(message.clone()),
        message: Some(format!("{} {}\r\n", code, message)),
    }
}

/// Translates user validation result to FTP response
pub fn user_result_to_ftp_response(result: UserValidationResult) -> CommandResult {
    CommandResult {
        status: CommandStatus::Success,
        message: Some("331 Password required\r\n".into()),
    }
}

/// Translates password validation result to FTP response
pub fn password_result_to_ftp_response(result: PasswordValidationResult) -> CommandResult {
    CommandResult {
        status: CommandStatus::Success,
        message: Some("230 Login successful\r\n".into()),
    }
}

/// Translates storage errors to FTP responses
pub fn storage_error_to_ftp_response(error: StorageError) -> CommandResult {
    let (code, message) = match error {
        StorageError::FileNotFound(p) => (550, format!("{}: File not found", p)),
        StorageError::DirectoryNotFound(p) => (550, format!("{}: Directory not found", p)),
        StorageError::PermissionDenied(p) => (550, format!("{}: Permission denied", p)),
        StorageError::InvalidPath(p) => (550, format!("Invalid path: {}", p)),
        StorageError::FileAlreadyExists(p) => (550, format!("{}: File already exists", p)),
        StorageError::NotADirectory(p) => (550, format!("{}: Not a directory", p)),
        StorageError::IoError(e) => (550, format!("I/O error: {}", e)),
        StorageError::PathTraversal(p) => (550, format!("Path traversal attempt: {}", p)),
        StorageError::UploadInProgress(p) => (550, format!("{}: Upload already in progress", p)),
    };

    CommandResult {
        status: CommandStatus::Failure(message.clone()),
        message: Some(format!("{} {}\r\n", code, message)),
    }
}

/// Translates list result to FTP response (initial response only)
pub fn list_result_to_ftp_response(result: ListResult) -> CommandResult {
    // The actual directory listing is sent over the data channel
    // This just returns the initial response
    CommandResult {
        status: CommandStatus::Success,
        message: Some("150 Here comes the directory listing\r\n".into()),
    }
}

/// Translates retrieve result to FTP response
pub fn retrieve_result_to_ftp_response(result: RetrieveResult) -> CommandResult {
    CommandResult {
        status: CommandStatus::Success,
        message: Some("150 Opening data connection for file transfer\r\n".into()),
    }
}

/// Translates store result to FTP response
pub fn store_result_to_ftp_response(result: StoreResult) -> CommandResult {
    CommandResult {
        status: CommandStatus::Success,
        message: Some("150 Ok to send data\r\n".into()),
    }
}

/// Translates delete result to FTP response
pub fn delete_result_to_ftp_response(result: DeleteResult) -> CommandResult {
    CommandResult {
        status: CommandStatus::Success,
        message: Some("250 File deleted successfully\r\n".into()),
    }
}

/// Translates transfer errors to FTP responses
pub fn transfer_error_to_ftp_response(error: TransferError) -> CommandResult {
    let (code, message) = match error {
        TransferError::DataChannelNotInitialized => {
            (425, "Data channel not initialized".to_string())
        }
        TransferError::PortBindingFailed(addr, e) => {
            (425, format!("Can't bind to {}: {}", addr, e))
        }
        TransferError::NoAvailablePort => (425, "No available port".to_string()),
        TransferError::ListenerConfigurationFailed(e) => {
            (425, format!("Listener config failed: {}", e))
        }
        TransferError::ConnectionTimeout(addr) => {
            (425, format!("Connection timeout from {}", addr))
        }
        TransferError::DataChannelSetupFailed(msg) => (425, msg),
        TransferError::InvalidPortCommand(msg) => (501, msg),
        TransferError::IpMismatch { expected, provided } => (
            501,
            format!("IP mismatch: expected {}, got {}", expected, provided),
        ),
        TransferError::InvalidPortRange(port) => (
            501,
            format!("Port {} out of range (must be 1024-65535)", port),
        ),
        TransferError::TransferFailed(e) => (426, format!("Transfer failed: {}", e)),
    };

    CommandResult {
        status: CommandStatus::Failure(message.clone()),
        message: Some(format!("{} {}\r\n", code, message)),
    }
}

/// Translates passive mode result to FTP response
pub fn passive_result_to_ftp_response(result: PassiveModeResult) -> CommandResult {
    CommandResult {
        status: CommandStatus::Success,
        message: Some(format!(
            "227 Entering Passive Mode ({})\r\n",
            result.data_socket
        )),
    }
}

/// Translates active mode result to FTP response
pub fn active_result_to_ftp_response(result: ActiveModeResult) -> CommandResult {
    CommandResult {
        status: CommandStatus::Success,
        message: Some("200 PORT command successful\r\n".into()),
    }
}

/// Translates navigate errors to FTP responses
pub fn navigate_error_to_ftp_response(error: NavigateError) -> CommandResult {
    let (code, message) = match error {
        NavigateError::InvalidPath(p) => (550, format!("Invalid path: {}", p)),
        NavigateError::DirectoryNotFound(p) => (550, format!("{}: Directory not found", p)),
        NavigateError::NotADirectory(p) => (550, format!("{}: Not a directory", p)),
        NavigateError::PermissionDenied(p) => (550, format!("{}: Permission denied", p)),
        NavigateError::PathTraversal(p) => (550, format!("Path traversal attempt: {}", p)),
    };

    CommandResult {
        status: CommandStatus::Failure(message.clone()),
        message: Some(format!("{} {}\r\n", code, message)),
    }
}

/// Translates PWD result to FTP response
pub fn pwd_result_to_ftp_response(result: PwdResult) -> CommandResult {
    CommandResult {
        status: CommandStatus::Success,
        message: Some(format!("257 \"{}\"\r\n", result.virtual_path)),
    }
}

/// Translates CWD result to FTP response
pub fn cwd_result_to_ftp_response(result: CwdResult) -> CommandResult {
    CommandResult {
        status: CommandStatus::Success,
        message: Some("250 Directory changed successfully\r\n".into()),
    }
}

/// Translates client errors to FTP responses
pub fn client_error_to_ftp_response(error: ClientError) -> CommandResult {
    let (code, message) = match error {
        ClientError::ClientNotFound(addr) => (421, format!("Client {} not found", addr)),
        ClientError::InvalidState(msg) => (530, msg),
    };

    CommandResult {
        status: CommandStatus::Failure(message.clone()),
        message: Some(format!("{} {}\r\n", code, message)),
    }
}


