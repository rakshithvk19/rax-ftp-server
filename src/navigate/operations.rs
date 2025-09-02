//! Navigation operations implementation

use crate::error::NavigateError;
use std::path::Path;

/// Changes the working directory for a client
pub fn change_directory(
    server_root: &Path,
    current_virtual_path: &str,
    target_path: &str,
) -> Result<String, NavigateError> {
    use crate::storage::validation::{resolve_cwd_path, virtual_to_real_path};
    
    // Validate target path
    if target_path.is_empty() {
        return Err(NavigateError::InvalidPath("Empty path provided".into()));
    }
    
    // Resolve the new virtual path
    let new_virtual_path = resolve_cwd_path(current_virtual_path, target_path)
        .map_err(|e| NavigateError::InvalidPath(e))?;
    
    // Convert to real path and verify it exists
    let real_path = virtual_to_real_path(server_root, &new_virtual_path);
    
    if !real_path.exists() {
        return Err(NavigateError::DirectoryNotFound(new_virtual_path));
    }
    
    if !real_path.is_dir() {
        return Err(NavigateError::NotADirectory(new_virtual_path));
    }
    
    // Additional security check to ensure path is within server root
    match real_path.canonicalize() {
        Ok(canonical_path) => {
            let server_root_canonical = server_root.canonicalize()
                .map_err(|_| NavigateError::InvalidPath("Server root invalid".into()))?;
            
            if !canonical_path.starts_with(&server_root_canonical) {
                return Err(NavigateError::PathTraversal(target_path.into()));
            }
        }
        Err(_) => {
            return Err(NavigateError::PermissionDenied(new_virtual_path));
        }
    }
    
    Ok(new_virtual_path)
}
