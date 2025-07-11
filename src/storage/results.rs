//! Storage result types
//!
//! Defines result structures returned by storage operations.

use std::path::PathBuf;

/// Result of a directory listing operation
#[derive(Debug, Clone)]
pub struct ListResult {
    pub entries: Vec<String>,
    pub path: String,
}

/// Result of a file retrieval operation
#[derive(Debug, Clone)]
pub struct RetrieveResult {
    pub file_path: PathBuf,
    pub virtual_path: String,
}

/// Result of a file storage operation
#[derive(Debug, Clone)]
pub struct StoreResult {
    pub file_path: PathBuf,
    pub virtual_path: String,
    pub temp_path: PathBuf,
}

/// Result of a file deletion operation
#[derive(Debug, Clone)]
pub struct DeleteResult {
    pub file_path: PathBuf,
    pub virtual_path: String,
}
