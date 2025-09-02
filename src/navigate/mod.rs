//! Navigate module
//!
//! Handles directory navigation operations for FTP clients,
//! including changing directories and retrieving current paths.

mod operations;

// Re-export public types and functions
pub use operations::change_directory;
