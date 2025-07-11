//! File system operations
//!
//! Handles file system operations for the FTP server.

use std::fs;
use std::io::Result;
use std::path::Path;

/// Create a directory
pub fn create_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
}

/// Check if file exists
pub fn file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// Check if directory exists
pub fn directory_exists(path: &Path) -> bool {
    path.exists() && path.is_dir()
}
