//! Navigate module
//!
//! Handles directory navigation operations for FTP clients,
//! including changing directories and retrieving current paths.

mod operations;
mod results;

// Re-export public types and functions
pub use operations::{change_directory, get_working_directory};
pub use results::{CwdResult, PwdResult};
