//! Storage module
//!
//! Handles file system operations and storage management.

pub mod filesystem;
mod operations;
pub mod permissions;
mod results;
pub mod validation;

pub use operations::{delete_file, list_directory, prepare_file_retrieval, prepare_file_storage};
pub use results::{DeleteResult, ListResult, RetrieveResult, StoreResult};
