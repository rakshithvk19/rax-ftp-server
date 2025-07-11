//! Error handlers
//!
//! Provides error handling and recovery functions.

use crate::error::types::FtpServerError;
use log::error;

/// Handle an FTP server error
pub fn handle_error(err: &FtpServerError) {
    error!("FTP Server Error: {}", err);
}

/// Convert error to FTP response code
pub fn error_to_ftp_code(err: &FtpServerError) -> u16 {
    match err {
        FtpServerError::Auth(_) => 530,
        FtpServerError::Storage(_) => 550,
        FtpServerError::Transfer(_) => 425,
        FtpServerError::Navigate(_) => 550,
        FtpServerError::Client(_) => 421,
        FtpServerError::IoError(_) => 550,
        FtpServerError::NetworkError(_) => 421,
        FtpServerError::ProtocolError(_) => 500,
        FtpServerError::FileSystemError(_) => 550,
    }
}
