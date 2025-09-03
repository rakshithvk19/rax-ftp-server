//! Path validation and sanitization
//!
//! Handles comprehensive path validation, security checks, and path resolution for FTP operations.

use log::warn;
use std::path::{Path, PathBuf};

/// Maximum allowed subdirectory depth (0 = root, 1 = one level deep, etc.)
pub const MAX_DIRECTORY_DEPTH: usize = 3;

/// Normalize path separators to Unix style and validate path structure
pub fn normalize_path(path: &str) -> Result<String, String> {
    if path.is_empty() {
        return Ok("/".to_string());
    }

    // Convert Windows-style backslashes to forward slashes
    let normalized = path.replace('\\', "/");

    // Remove consecutive slashes
    let normalized = normalized
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/");

    // Ensure leading slash for absolute paths
    if path.starts_with('/') || path.starts_with('\\') {
        Ok(format!("/{normalized}"))
    } else {
        Ok(normalized)
    }
}

/// Validate directory depth doesn't exceed maximum allowed
pub fn validate_directory_depth(path: &str) -> Result<(), String> {
    let depth = if path == "/" {
        0
    } else {
        path.trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .count()
    };

    if depth > MAX_DIRECTORY_DEPTH {
        return Err(format!(
            "Directory depth {depth} exceeds maximum allowed depth of {MAX_DIRECTORY_DEPTH}"
        ));
    }

    Ok(())
}

/// Validate that a path component doesn't contain dangerous characters
pub fn validate_path_component(component: &str) -> Result<(), String> {
    if component.is_empty() {
        return Err("Empty path component".to_string());
    }

    // Check for directory traversal
    if component == ".." || component == "." {
        return Err("Directory traversal not allowed".to_string());
    }

    // Check for dangerous characters
    let dangerous_chars = ['\0', '<', '>', '|', '"', '*', '?', ':'];
    for ch in dangerous_chars {
        if component.contains(ch) {
            return Err(format!("Invalid character '{ch}' in path"));
        }
    }

    // Check for reserved names on Windows
    let reserved_names = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    if reserved_names.contains(&component.to_uppercase().as_str()) {
        return Err(format!("Reserved filename: {component}"));
    }

    Ok(())
}

/// Comprehensive path validation
pub fn validate_path(path: &str) -> Result<String, String> {
    // Step 1: Normalize path separators
    let normalized = normalize_path(path)?;

    // Step 2: Validate directory depth
    validate_directory_depth(&normalized)?;

    // Step 3: Validate each path component
    if normalized != "/" {
        let components: Vec<&str> = normalized
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        for component in components {
            validate_path_component(component)?;
        }
    }

    Ok(normalized)
}

/// Resolve a file path relative to current virtual directory
pub fn resolve_file_path(current_virtual_path: &str, file_path: &str) -> Result<String, String> {
    let file_path = file_path.trim();

    if file_path.is_empty() {
        return Err("Empty file path".to_string());
    }

    // Determine the virtual file path
    let virtual_file_path = if file_path.starts_with('/') || file_path.starts_with('\\') {
        // Absolute path
        validate_path(file_path)?
    } else {
        // Relative path - resolve relative to current virtual directory
        let combined = if current_virtual_path.ends_with('/') {
            format!("{current_virtual_path}{file_path}")
        } else {
            format!("{current_virtual_path}/{file_path}")
        };
        validate_path(&combined)?
    };

    Ok(virtual_file_path)
}

/// Resolve a directory path for CWD command
pub fn resolve_cwd_path(
    current_virtual_path: &str,
    requested_path: &str,
) -> Result<String, String> {
    let requested = requested_path.trim();

    if requested.is_empty() {
        return Ok(current_virtual_path.to_string());
    }

    // Handle special case of ".." when already at root
    if requested == ".." && current_virtual_path == "/" {
        return Ok("/".to_string());
    }

    // Handle absolute paths
    if requested.starts_with('/') || requested.starts_with('\\') {
        return validate_path(requested);
    }

    // Handle relative paths
    let combined = if current_virtual_path.ends_with('/') {
        format!("{current_virtual_path}{requested}")
    } else {
        format!("{current_virtual_path}/{requested}")
    };

    validate_path(&combined)
}

/// Convert virtual path to real filesystem path within server_root
pub fn virtual_to_real_path(server_root: &Path, virtual_path: &str) -> PathBuf {
    let mut real_path = server_root.to_path_buf();

    // Remove leading slash and add to server_root
    let relative_path = virtual_path.trim_start_matches('/');
    if !relative_path.is_empty() {
        real_path.push(relative_path);
    }

    real_path
}

/// Verify real path is within server_root bounds (security check)
pub fn verify_path_within_bounds(server_root: &Path, real_path: &Path) -> Result<(), String> {
    match real_path.canonicalize() {
        Ok(canonical_real) => {
            match server_root.canonicalize() {
                Ok(canonical_root) => {
                    if !canonical_real.starts_with(canonical_root) {
                        return Err("Path outside server root".to_string());
                    }
                }
                Err(_) => {
                    // If we can't canonicalize root, skip this check
                    warn!("Could not canonicalize server root path");
                }
            }
        }
        Err(_) => {
            // Path doesn't exist yet, check parent directory
            if let Some(parent) = real_path.parent() {
                if let Ok(canonical_parent) = parent.canonicalize() {
                    if let Ok(canonical_root) = server_root.canonicalize() {
                        if !canonical_parent.starts_with(canonical_root) {
                            return Err("Path outside server root".to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Complete file path resolution with all security checks
pub fn resolve_and_validate_file_path(
    server_root: &Path,
    current_virtual_path: &str,
    file_path: &str,
) -> Result<(PathBuf, String), String> {
    // Resolve virtual file path
    let virtual_file_path = resolve_file_path(current_virtual_path, file_path)?;

    // Convert to real path
    let real_path = virtual_to_real_path(server_root, &virtual_file_path);

    // Verify security bounds
    verify_path_within_bounds(server_root, &real_path)?;

    Ok((real_path, virtual_file_path))
}
