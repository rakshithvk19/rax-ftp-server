//! Path validation
//! 
//! Handles path validation and security checks.

use std::path::Path;

/// Validate that a path is safe (no directory traversal)
pub fn is_safe_path(path: &Path) -> bool {
    // Check for directory traversal attempts
    !path.to_string_lossy().contains("..")
}

/// Sanitize a filename
pub fn sanitize_filename(filename: &str) -> Option<String> {
    if filename.is_empty() || filename.contains("..") {
        None
    } else {
        Some(filename.to_string())
    }
}
