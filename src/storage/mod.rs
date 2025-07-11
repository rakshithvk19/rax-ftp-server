//! File system storage management
//!
//! Handles file operations, permissions, and path validation.

pub mod filesystem;
pub mod permissions;
pub mod validation;

// Re-export commonly used validation functions
pub use validation::{MAX_DIRECTORY_DEPTH, resolve_and_validate_file_path, virtual_to_real_path};
