//! Storage operations
//!
//! Handles file system operations for FTP commands including list, retrieve, store, and delete.

use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;
use log::{error, info};

use crate::error::StorageError;
use crate::storage::results::{ListResult, RetrieveResult, StoreResult, DeleteResult};
use crate::storage::validation::{resolve_and_validate_file_path, virtual_to_real_path};

/// Lists the contents of a directory
pub fn list_directory(
    server_root: &Path,
    current_virtual_path: &str,
) -> Result<ListResult, StorageError> {
    let real_path = virtual_to_real_path(server_root, current_virtual_path);
    
    // Read directory contents with retries
    let retries = 3;
    let mut result = None;
    
    for attempt in 1..=retries {
        match fs::read_dir(&real_path) {
            Ok(entries) => {
                let mut file_list = vec![];
                
                // Add . and .. entries first
                file_list.push(".".to_string());
                if current_virtual_path != "/" {
                    file_list.push("..".to_string());
                }
                
                // Add regular files and directories
                for entry in entries.flatten() {
                    file_list.push(entry.file_name().to_string_lossy().to_string());
                }
                
                result = Some(file_list);
                break;
            }
            Err(e) => {
                if attempt < retries && e.kind() == std::io::ErrorKind::PermissionDenied {
                    thread::sleep(Duration::from_millis(100 * attempt as u64));
                    continue;
                } else {
                    error!(
                        "Failed to list directory {} (real: {}): {}",
                        current_virtual_path,
                        real_path.display(),
                        e
                    );
                    return Err(StorageError::from(e));
                }
            }
        }
    }
    
    let entries = result.ok_or_else(|| {
        StorageError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to read directory after retries",
        ))
    })?;
    
    info!(
        "Listed directory {} (real: {}) - {} entries",
        current_virtual_path,
        real_path.display(),
        entries.len()
    );
    
    Ok(ListResult {
        entries,
        path: current_virtual_path.to_string(),
    })
}

/// Prepares for file retrieval
pub fn prepare_file_retrieval(
    server_root: &Path,
    current_virtual_path: &str,
    filename: &str,
) -> Result<RetrieveResult, StorageError> {
    if filename.is_empty() {
        return Err(StorageError::InvalidPath("Empty filename".into()));
    }
    
    let (file_path, virtual_file_path) = resolve_and_validate_file_path(
        server_root,
        current_virtual_path,
        filename,
    ).map_err(|e| StorageError::InvalidPath(e))?;
    
    // Check if file exists
    if !file_path.exists() {
        return Err(StorageError::FileNotFound(virtual_file_path));
    }
    
    if !file_path.is_file() {
        return Err(StorageError::NotADirectory(virtual_file_path));
    }
    
    info!(
        "Prepared file retrieval for {} (virtual: {}, real: {})",
        filename,
        virtual_file_path,
        file_path.display()
    );
    
    Ok(RetrieveResult {
        file_path,
        virtual_path: virtual_file_path,
    })
}

/// Prepares for file storage
pub fn prepare_file_storage(
    server_root: &Path,
    current_virtual_path: &str,
    filename: &str,
) -> Result<StoreResult, StorageError> {
    if filename.is_empty() {
        return Err(StorageError::InvalidPath("Empty filename".into()));
    }
    
    let (file_path, virtual_file_path) = resolve_and_validate_file_path(
        server_root,
        current_virtual_path,
        filename,
    ).map_err(|e| StorageError::InvalidPath(e))?;
    
    // Check if parent directory exists
    if let Some(parent_dir) = file_path.parent() {
        if !parent_dir.exists() {
            return Err(StorageError::DirectoryNotFound(
                parent_dir.to_string_lossy().to_string()
            ));
        }
        if !parent_dir.is_dir() {
            return Err(StorageError::NotADirectory(
                parent_dir.to_string_lossy().to_string()
            ));
        }
    }
    
    // Check if file already exists
    if file_path.exists() {
        return Err(StorageError::FileAlreadyExists(virtual_file_path));
    }
    
    // Create temporary file path
    let temp_file_path = file_path.with_extension(format!(
        "{}.tmp",
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
    ));
    
    // Check if temporary file exists (upload in progress)
    if temp_file_path.exists() {
        return Err(StorageError::UploadInProgress(virtual_file_path));
    }
    
    info!(
        "Prepared file storage for {} (virtual: {}, real: {})",
        filename,
        virtual_file_path,
        file_path.display()
    );
    
    Ok(StoreResult {
        file_path,
        virtual_path: virtual_file_path,
        temp_path: temp_file_path,
    })
}

/// Deletes a file
pub fn delete_file(
    server_root: &Path,
    current_virtual_path: &str,
    filename: &str,
) -> Result<DeleteResult, StorageError> {
    if filename.is_empty() {
        return Err(StorageError::InvalidPath("Empty filename".into()));
    }
    
    let (file_path, virtual_file_path) = resolve_and_validate_file_path(
        server_root,
        current_virtual_path,
        filename,
    ).map_err(|e| StorageError::InvalidPath(e))?;
    
    // Verify file exists
    if !file_path.exists() {
        return Err(StorageError::FileNotFound(virtual_file_path));
    }
    
    if !file_path.is_file() {
        return Err(StorageError::NotADirectory(virtual_file_path));
    }
    
    // Delete with retries for permission issues
    let retries = 3;
    for attempt in 1..=retries {
        match fs::remove_file(&file_path) {
            Ok(_) => {
                info!(
                    "Deleted file {} (virtual: {}, real: {})",
                    filename,
                    virtual_file_path,
                    file_path.display()
                );
                return Ok(DeleteResult {
                    file_path,
                    virtual_path: virtual_file_path,
                });
            }
            Err(e) => {
                if attempt < retries && e.kind() == std::io::ErrorKind::PermissionDenied {
                    thread::sleep(Duration::from_millis(100 * attempt as u64));
                    continue;
                } else {
                    error!(
                        "Failed to delete file {} (virtual: {}, real: {}): {}",
                        filename,
                        virtual_file_path,
                        file_path.display(),
                        e
                    );
                    return Err(StorageError::from(e));
                }
            }
        }
    }
    
    Err(StorageError::IoError(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Failed to delete file after retries",
    )))
}
