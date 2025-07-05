//! FTP Response handling
//! 
//! Defines FTP response codes and formatting.

/// Standard FTP response codes
pub const OK: u16 = 200;
pub const READY: u16 = 220;
pub const LOGIN_SUCCESS: u16 = 230;
pub const TRANSFER_COMPLETE: u16 = 226;
pub const PASSWORD_REQUIRED: u16 = 331;
pub const AUTH_FAILED: u16 = 530;
pub const FILE_NOT_FOUND: u16 = 550;

/// Format an FTP response message
pub fn format_response(code: u16, message: &str) -> String {
    format!("{} {}\r\n", code, message)
}
