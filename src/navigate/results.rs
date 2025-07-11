//! Result types for navigate operations

use std::path::PathBuf;

/// Result of a PWD (print working directory) operation
#[derive(Debug, Clone)]
pub struct PwdResult {
    pub virtual_path: String,
}

/// Result of a CWD (change working directory) operation
#[derive(Debug, Clone)]
pub struct CwdResult {
    pub new_virtual_path: String,
    pub real_path: PathBuf,
}
