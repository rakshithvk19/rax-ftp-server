//! Command handlers module for the Rax FTP server.
//!
//! This module defines handler functions for FTP commands, handling
//! authentication, file operations, directory management, and data channel
//! setup per client connection.

use crate::auth;
use crate::transfer::{ChannelEntry, ChannelRegistry};
use crate::client::Client;
use crate::protocol::{Command, CommandData, CommandResult, CommandStatus};
use crate::transfer::setup_data_stream;
use crate::transfer::{handle_file_download, handle_file_upload};
use crate::server::config::ServerConfig;
use log::{error, info};

use std::env;
use std::fs;
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

/// Helper function to normalize a path by removing redundant components.
/// Handles: double slashes, current directory (.), but rejects parent directory (..) for security.
fn normalize_virtual_path(path: &str) -> Result<String, String> {
    let path = path.trim();
    
    // Handle empty path
    if path.is_empty() {
        return Ok("/".to_string());
    }
    
    // Start with root if not absolute
    let mut normalized = if path.starts_with('/') {
        String::new()
    } else {
        String::new()
    };
    
    let components: Vec<&str> = path
        .split('/')
        .filter(|comp| !comp.is_empty() && *comp != ".")
        .collect();
    
    for component in components {
        // Reject directory traversal attempts
        if component == ".." {
            return Err("Directory traversal not allowed".to_string());
        }
        
        // Reject components with dangerous characters
        if component.contains('\0') || component.contains('\\') {
            return Err("Invalid path characters".to_string());
        }
        
        normalized.push('/');
        normalized.push_str(component);
    }
    
    // Ensure we always return at least "/"
    if normalized.is_empty() {
        normalized = "/".to_string();
    }
    
    Ok(normalized)
}

/// Resolve a CWD path relative to the current virtual directory.
fn resolve_cwd_path(current_virtual_path: &str, requested_path: &str) -> Result<String, String> {
    let requested = requested_path.trim();
    
    if requested.is_empty() {
        return Ok(current_virtual_path.to_string());
    }
    
    // Handle absolute paths
    if requested.starts_with('/') {
        return normalize_virtual_path(requested);
    }
    
    // Handle relative paths - concatenate with current path
    let combined = if current_virtual_path.ends_with('/') {
        format!("{}{}", current_virtual_path, requested)
    } else {
        format!("{}/{}", current_virtual_path, requested)
    };
    
    normalize_virtual_path(&combined)
}

/// Convert virtual path to real filesystem path within server_root.
fn virtual_to_real_path(server_root: &Path, virtual_path: &str) -> PathBuf {
    let mut real_path = server_root.to_path_buf();
    
    // Remove leading slash and add to server_root
    let relative_path = virtual_path.trim_start_matches('/');
    if !relative_path.is_empty() {
        real_path.push(relative_path);
    }
    
    real_path
}

/// Helper function to resolve a file path within the current virtual directory.
/// Converts virtual paths to real paths and ensures they stay within server_root.
fn resolve_file_path(server_root: &Path, current_virtual_path: &str, file_path: &str) -> Result<(PathBuf, String), String> {
    let file_path = file_path.trim();
    
    // Determine the virtual file path
    let virtual_file_path = if file_path.starts_with('/') {
        // Absolute virtual path
        normalize_virtual_path(file_path)?
    } else {
        // Relative path - resolve relative to current virtual directory
        let combined = if current_virtual_path.ends_with('/') {
            format!("{}{}", current_virtual_path, file_path)
        } else {
            format!("{}/{}", current_virtual_path, file_path)
        };
        normalize_virtual_path(&combined)?
    };
    
    // Convert to real path
    let real_path = virtual_to_real_path(server_root, &virtual_file_path);
    
    // Ensure the real path is within server_root (security check)
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
                }
            }
        }
        Err(_) => {
            // Path doesn't exist yet, but that's okay for file creation
            // Just verify the parent directory would be within bounds
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
    
    Ok((real_path, virtual_file_path))
}

/// Dispatches a received FTP command to its corresponding handler.
///
/// # Arguments
///
/// * `client` - Mutable reference to the client sending the command.
/// * `command` - Reference to the parsed FTP command enum.
/// * `channel_registry` - Mutable reference to the global channel registry.
///
/// # Returns
///
/// * `CommandResult` - Result of the command execution, including status and message.
pub fn handle_command(
    client: &mut Client,
    command: &Command,
    channel_registry: &mut ChannelRegistry,
    config: &ServerConfig,
) -> CommandResult {
    match command {
        Command::QUIT => handle_cmd_quit(client),
        Command::USER(username) => handle_cmd_user(client, username),
        Command::PASS(password) => handle_cmd_pass(client, password),
        Command::LIST => handle_cmd_list(client, config, channel_registry),
        Command::PWD => handle_cmd_pwd(client, config),
        Command::LOGOUT => handle_cmd_logout(client),
        Command::RETR(filename) => handle_cmd_retr(client, filename, channel_registry, config),
        Command::STOR(filename) => handle_cmd_stor(client, filename, channel_registry, config),
        Command::DEL(filename) => handle_cmd_del(client, filename, config),
        Command::CWD(path) => handle_cmd_cwd(client, path, config),
        Command::PASV => handle_cmd_pasv(client, channel_registry),
        Command::PORT(addr) => handle_cmd_port(client, channel_registry, addr),
        Command::RAX => handle_cmd_rax(),
        Command::UNKNOWN => handle_cmd_unknown(),
    }
}

pub fn handle_auth_command(client: &mut Client, command: &Command) -> CommandResult{
    match command {
        Command::USER(username) => handle_cmd_user(client, username),
        Command::PASS(password) => handle_cmd_pass(client, password),
        _ => CommandResult {
            status: CommandStatus::Failure("Authentication required".into()),
            message: Some("530 Please login with USER and PASS\r\n".into()),
            data: None,
        },
    }
}

/// Handles the QUIT command: logs out the client and signals connection close.
fn handle_cmd_quit(client: &mut Client) -> CommandResult {
    client.logout();

    CommandResult {
        status: CommandStatus::CloseConnection,
        message: Some("221 Goodbye\r\n".into()),
        data: None,
    }
}

/// Handles the USER command: validates username and sets client state accordingly.
///
/// Returns success response if valid; failure otherwise.
fn handle_cmd_user(client: &mut Client, username: &str) -> CommandResult {
    match auth::validate_user(username) {
        Ok(response) => {
            client.set_user_valid(true);
            client.set_logged_in(false);
            client.set_username(Some(username.to_string()));
            CommandResult {
                status: CommandStatus::Success,
                message: Some(response.into()),
                data: None,
            }
        }
        Err(e) => {
            client.set_user_valid(false);
            client.set_logged_in(false);
            client.set_username(None);
            CommandResult {
                status: CommandStatus::Failure(e.message().to_string()),
                message: Some(format!("{} {}\r\n", e.ftp_response(), e.message())),
                data: None,
            }
        }
    }
}

/// Handles the PASS command: validates password if username was previously validated.
///
/// Returns success if password matches; failure otherwise.
fn handle_cmd_pass(client: &mut Client, password: &str) -> CommandResult {
    if client.is_user_valid() {
        if let Some(username) = &client.username() {
            match auth::validate_password(username, password) {
                Ok(response) => {
                    client.set_logged_in(true);
                    return CommandResult {
                        status: CommandStatus::Success,
                        message: Some(response.into()),
                        data: None,
                    };
                }
                Err(e) => {
                    client.set_logged_in(false);
                    return CommandResult {
                        status: CommandStatus::Failure(e.message().to_string()),
                        message: Some(format!("{} {}\r\n", e.ftp_response(), e.message())),
                        data: None,
                    };
                }
            }
        }
    }
    // Username not set or invalid
    CommandResult {
        status: CommandStatus::Failure("Username not provided".into()),
        message: Some("530 Please enter the username first\r\n".into()),
        data: None,
    }
}

/// Handles the LIST command: provides directory listing to logged-in clients.
///
/// Establishes a data connection and sends the listing over the data channel.
fn handle_cmd_list(
    client: &mut Client, 
    config: &ServerConfig, 
    channel_registry: &mut ChannelRegistry
) -> CommandResult {
    use std::io::Write;
    
    // 1. Authentication check
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    // 2. Data channel initialization check
    if !client.is_data_channel_init() {
        return CommandResult {
            status: CommandStatus::Failure("Data channel not initialized".into()),
            message: Some("530 Data channel not initialized\r\n".into()),
            data: None,
        };
    }

    // 3. Get client address
    let client_addr = match client.client_addr() {
        Some(addr) => *addr,
        None => {
            return CommandResult {
                status: CommandStatus::Failure("Client address unknown".into()),
                message: Some("500 Internal server error\r\n".into()),
                data: None,
            };
        }
    };

    // 4. Convert virtual path to real path
    let real_path = virtual_to_real_path(&config.server_root, client.current_virtual_path());

    // 5. Read directory contents with retries
    let retries = 3;
    let file_list = {
        let mut result = None;
        for attempt in 1..=retries {
            match fs::read_dir(&real_path) {
                Ok(entries) => {
                    let mut file_list = vec![];
                    
                    // Add . and .. entries first
                    file_list.push(".".to_string());
                    if client.current_virtual_path() != "/" {
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
                            client.current_virtual_path(),
                            real_path.display(),
                            e
                        );
                        return CommandResult {
                            status: CommandStatus::Failure(e.to_string()),
                            message: Some("550 Failed to list directory\r\n".into()),
                            data: None,
                        };
                    }
                }
            }
        }
        result.unwrap_or_else(Vec::new)
    };

    info!(
        "Client {} listed directory {} (real: {}) - {} entries",
        client_addr,
        client.current_virtual_path(),
        real_path.display(),
        file_list.len()
    );

    // 6. Setup data stream for directory listing
    let mut data_stream = match setup_data_stream(channel_registry, &client_addr) {
        Some(stream) => stream,
        None => {
            error!(
                "Failed to establish data connection for client {}",
                client_addr
            );
            return CommandResult {
                status: CommandStatus::Failure("425 Can't open data connection".into()),
                message: Some("425 Can't open data connection\r\n".into()),
                data: None,
            };
        }
    };

    // 7. Send directory listing over data connection
    let listing_data = file_list.join("\r\n") + "\r\n";
    match data_stream.write_all(listing_data.as_bytes()) {
        Ok(_) => {
            match data_stream.flush() {
                Ok(_) => {
                    info!("Directory listing sent successfully to client {}", client_addr);
                    CommandResult {
                        status: CommandStatus::Success,
                        message: Some("226 Directory send OK\r\n".into()),
                        data: None,
                    }
                }
                Err(e) => {
                    error!("Failed to flush data stream: {}", e);
                    CommandResult {
                        status: CommandStatus::Failure("426 Connection closed; transfer aborted".into()),
                        message: Some("426 Connection closed; transfer aborted\r\n".into()),
                        data: None,
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to send directory listing: {}", e);
            CommandResult {
                status: CommandStatus::Failure("426 Connection closed; transfer aborted".into()),
                message: Some("426 Connection closed; transfer aborted\r\n".into()),
                data: None,
            }
        }
    }
}

/// Handles the LOGOUT command: logs out the client if currently logged in.
fn handle_cmd_logout(client: &mut Client) -> CommandResult {
    if client.is_logged_in() {
        client.logout();
        CommandResult {
            status: CommandStatus::Success,
            message: Some("221 Logout successful\r\n".into()),
            data: None,
        }
    } else {
        CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 User Not logged in\r\n".into()),
            data: None,
        }
    }
}

/// Handles the STOR command: uploads a file from client to server.
///
/// Performs client authentication and filename validation,
/// establishes a data channel, then delegates to file upload handler.
///
/// Returns status and message describing the outcome.
pub fn handle_cmd_stor(
    client: &mut Client,
    filename: &str,
    channel_registry: &mut ChannelRegistry,
    config: &ServerConfig,
) -> CommandResult {
    use log::{error, info};
    use std::fs;

    // 1. Authentication check
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    // 2. Data channel initialization check
    if !client.is_data_channel_init() {
        return CommandResult {
            status: CommandStatus::Failure("Data channel not initialized".into()),
            message: Some("530 Data channel not initialized\r\n".into()),
            data: None,
        };
    }

    // 3. Filename presence check
    if filename.is_empty() {
        return CommandResult {
            status: CommandStatus::Failure("Missing filename".into()),
            message: Some("501 Syntax error in parameters or arguments\r\n".into()),
            data: None,
        };
    }

    // 4. Filename sanitization to prevent directory traversal and invalid characters
    if filename.contains("..")
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains(':')
        || filename.contains('*')
        || filename.contains('?')
        || filename.contains('"')
        || filename.contains('<')
        || filename.contains('>')
        || filename.contains('|')
    {
        return CommandResult {
            status: CommandStatus::Failure("Invalid filename".into()),
            message: Some("550 Filename invalid\r\n".into()),
            data: None,
        };
    }

    // 5. Resolve file path within current virtual directory
    let (file_path, virtual_file_path) = match resolve_file_path(&config.server_root, client.current_virtual_path(), filename) {
        Ok((real_path, virtual_path)) => (real_path, virtual_path),
        Err(e) => {
            error!("STOR path resolution error: {}", e);
            return CommandResult {
                status: CommandStatus::Failure("Invalid file path".into()),
                message: Some("550 Invalid file path\r\n".into()),
                data: None,
            };
        }
    };

    // 6. Check if file already exists
    if fs::metadata(&file_path).is_ok() {
        return CommandResult {
            status: CommandStatus::Failure("File exists".into()),
            message: Some(format!("550 {}: File already exists\r\n", virtual_file_path)),
            data: None,
        };
    }

    // 7. Retrieve client address for logging
    let client_addr = match client.client_addr() {
        Some(addr) => addr,
        None => {
            return CommandResult {
                status: CommandStatus::Failure("Client address unknown".into()),
                message: Some("500 Internal server error\r\n".into()),
                data: None,
            };
        }
    };

    info!(
        "Client {} requested to store {} (virtual: {}, real: {})",
        client_addr, filename, virtual_file_path, file_path.display()
    );

    // 8. Setup data stream for file upload
    let data_stream = match setup_data_stream(channel_registry, client_addr) {
        Some(stream) => stream,
        None => {
            error!(
                "Failed to establish data connection for client {}",
                client_addr
            );
            return CommandResult {
                status: CommandStatus::Failure("425 Can't open data connection\r\n".into()),
                message: Some("425 Can't open data connection\r\n".into()),
                data: None,
            };
        }
    };

    // 9. Delegate file upload to file transfer module
    match handle_file_upload(data_stream, &file_path.to_string_lossy()) {
        Ok((status, msg)) => CommandResult {
            status,
            message: Some(msg.into()),
            data: None,
        },
        Err((status, msg)) => CommandResult {
            status,
            message: Some(msg.into()),
            data: None,
        },
    }
}

/// Handles the RETR command: downloads a file from server to client.
///
/// Performs client authentication and filename validation,
/// establishes a data channel, then delegates to file download handler.
///
/// Returns status and message describing the outcome.
fn handle_cmd_retr(
    client: &mut Client,
    filename: &str,
    channel_registry: &mut ChannelRegistry,
    config: &ServerConfig,
) -> CommandResult {
    // 1. Authentication check
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    // 2. Data channel initialized check
    if !client.is_data_channel_init() {
        return CommandResult {
            status: CommandStatus::Failure("Data channel not initialized".into()),
            message: Some("530 Data channel not initialized\r\n".into()),
            data: None,
        };
    }

    // 3. Filename presence check
    if filename.is_empty() {
        return CommandResult {
            status: CommandStatus::Failure("Missing filename".into()),
            message: Some("501 Syntax error in parameters or arguments\r\n".into()),
            data: None,
        };
    }

    // 4. Filename sanitization
    if filename.contains("..")
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains(':')
        || filename.contains('*')
        || filename.contains('?')
        || filename.contains('"')
        || filename.contains('<')
        || filename.contains('>')
        || filename.contains('|')
    {
        return CommandResult {
            status: CommandStatus::Failure("Invalid filename".into()),
            message: Some("550 Filename invalid\r\n".into()),
            data: None,
        };
    }

    // 5. Resolve file path within current virtual directory
    let (file_path, virtual_file_path) = match resolve_file_path(&config.server_root, client.current_virtual_path(), filename) {
        Ok((real_path, virtual_path)) => (real_path, virtual_path),
        Err(e) => {
            error!("RETR path resolution error: {}", e);
            return CommandResult {
                status: CommandStatus::Failure("Invalid file path".into()),
                message: Some("550 Invalid file path\r\n".into()),
                data: None,
            };
        }
    };

    // 6. Check if file exists
    if fs::metadata(&file_path).is_err() {
        return CommandResult {
            status: CommandStatus::Failure("File not found".into()),
            message: Some(format!("550 {}: File not found\r\n", virtual_file_path)),
            data: None,
        };
    }

    // 7. Retrieve client address
    let client_addr = match client.client_addr() {
        Some(addr) => addr,
        None => {
            return CommandResult {
                status: CommandStatus::Failure("Client address unknown".into()),
                message: Some("500 Internal server error\r\n".into()),
                data: None,
            };
        }
    };

    info!(
        "Client {} requested to retrieve {} (virtual: {}, real: {})",
        client_addr, filename, virtual_file_path, file_path.display()
    );

    // 8. Setup data stream for file download
    let data_stream = match setup_data_stream(channel_registry, client_addr) {
        Some(stream) => stream,
        None => {
            error!(
                "Failed to establish data connection for client {}",
                client_addr
            );
            return CommandResult {
                status: CommandStatus::Failure("425 Can't open data connection\r\n".into()),
                message: Some("425 Can't open data connection\r\n".into()),
                data: None,
            };
        }
    };

    // 9. Delegate file download to file transfer module
    match handle_file_download(data_stream, &file_path.to_string_lossy()) {
        Ok((status, msg)) => CommandResult {
            status,
            message: Some(msg.into()),
            data: None,
        },
        Err((status, msg)) => CommandResult {
            status,
            message: Some(msg.into()),
            data: None,
        },
    }
}

/// Handles the DEL command: deletes a specified file on the server.
///
/// Checks authentication and file presence before deletion.
fn handle_cmd_del(client: &Client, filename: &str, config: &ServerConfig) -> CommandResult {
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    if filename.is_empty() {
        return CommandResult {
            status: CommandStatus::Failure("Missing filename".into()),
            message: Some("501 Syntax error in parameters or arguments\r\n".into()),
            data: None,
        };
    }

    // Resolve file path within current virtual directory
    let (file_path, virtual_file_path) = match resolve_file_path(&config.server_root, client.current_virtual_path(), filename) {
        Ok((real_path, virtual_path)) => (real_path, virtual_path),
        Err(e) => {
            error!("DEL path resolution error: {}", e);
            return CommandResult {
                status: CommandStatus::Failure("Invalid file path".into()),
                message: Some("550 Invalid file path\r\n".into()),
                data: None,
            };
        }
    };

    let retries = 3;
    for attempt in 1..=retries {
        match fs::remove_file(&file_path) {
            Ok(_) => {
                info!(
                    "Client {} deleted file {} (virtual: {}, real: {})",
                    client.client_addr().map(|a| a.to_string()).unwrap_or_else(|| "unknown".to_string()),
                    filename,
                    virtual_file_path,
                    file_path.display()
                );
                return CommandResult {
                    status: CommandStatus::Success,
                    message: Some("250 File deleted successfully\r\n".into()),
                    data: None,
                };
            }
            Err(e) => {
                if attempt < retries && e.kind() == std::io::ErrorKind::PermissionDenied {
                    thread::sleep(Duration::from_millis(100 * attempt as u64));
                    continue;
                } else {
                    error!(
                        "Failed to delete file {} (virtual: {}, real: {}): {}",
                        filename, virtual_file_path, file_path.display(), e
                    );
                    return CommandResult {
                        status: CommandStatus::Failure(e.to_string()),
                        message: Some(format!("550 {}: Failed to delete file\r\n", virtual_file_path)),
                        data: None,
                    };
                }
            }
        }
    }

    CommandResult {
        status: CommandStatus::Failure("Unexpected error".into()),
        message: Some("550 Internal server error\r\n".into()),
        data: None,
    }
}

/// Handles the CWD command: changes the client's virtual working directory.
/// Validates that the target directory exists within server_root and updates client state.
fn handle_cmd_cwd(client: &mut Client, path: &str, config: &ServerConfig) -> CommandResult {
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    if path.is_empty() {
        return CommandResult {
            status: CommandStatus::Failure("Missing directory path".into()),
            message: Some("501 Syntax error in parameters or arguments\r\n".into()),
            data: None,
        };
    }

    // Handle special case of ".." when already at root
    if path.trim() == ".." && client.current_virtual_path() == "/" {
        // Stay at root, don't return error
        return CommandResult {
            status: CommandStatus::Success,
            message: Some("250 Directory changed successfully\r\n".into()),
            data: None,
        };
    }

    // Resolve the new virtual path
    let new_virtual_path = match resolve_cwd_path(client.current_virtual_path(), path) {
        Ok(path) => path,
        Err(e) => {
            error!("CWD path resolution error: {}", e);
            return CommandResult {
                status: CommandStatus::Failure("Invalid path".into()),
                message: Some("550 Invalid path\r\n".into()),
                data: None,
            };
        }
    };

    // Convert to real path and check if directory exists
    let real_path = virtual_to_real_path(&config.server_root, &new_virtual_path);
    
    if !real_path.exists() {
        return CommandResult {
            status: CommandStatus::Failure("Directory not found".into()),
            message: Some(format!("550 {}: Directory not found\r\n", new_virtual_path)),
            data: None,
        };
    }

    if !real_path.is_dir() {
        return CommandResult {
            status: CommandStatus::Failure("Not a directory".into()),
            message: Some(format!("550 {}: Not a directory\r\n", new_virtual_path)),
            data: None,
        };
    }

    // Update client's virtual path
    client.set_current_virtual_path(new_virtual_path.clone());
    
    info!(
        "Client {} changed directory to {} (real: {})",
        client.client_addr().map(|a| a.to_string()).unwrap_or_else(|| "unknown".to_string()),
        new_virtual_path,
        real_path.display()
    );

    CommandResult {
        status: CommandStatus::Success,
        message: Some("250 Directory changed successfully\r\n".into()),
        data: None,
    }
}

/// Handles the PWD command: returns the current virtual directory to the client.
///
/// Returns the client's current virtual directory path.
fn handle_cmd_pwd(client: &Client, config: &ServerConfig) -> CommandResult {
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    // Return the client's current virtual path
    let virtual_path = client.current_virtual_path();
    CommandResult {
        status: CommandStatus::Success,
        message: Some(format!("257 \"{}\"\r\n", virtual_path)),
        data: None,
    }
}

/// Handles the PASV command: sets up passive FTP mode.
///
/// Binds a listener on an available data socket, updates the registry,
/// and returns the PASV response with socket info to the client.
fn handle_cmd_pasv(client: &mut Client, channel_registry: &mut ChannelRegistry) -> CommandResult {
    let client_addr = *client.client_addr().unwrap();

    // Ensure client is authenticated
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    // Prevent duplicate data channel initialization
    if channel_registry.contains(&client_addr) {
        return CommandResult {
            status: CommandStatus::Failure("Data channel already initialized".into()),
            message: Some("425 Data connection already initialized\r\n".into()),
            data: None,
        };
    }

    // Find next available socket for data connection
    if let Some(data_socket) = channel_registry.next_available_socket() {
        match TcpListener::bind(data_socket) {
            Ok(listener) => {
                // Set listener to non-blocking to avoid blocking main thread
                if let Err(e) = listener.set_nonblocking(true) {
                    error!("Failed to set non-blocking mode: {}", e);
                    return CommandResult {
                        status: CommandStatus::Failure("Failed to configure listener".into()),
                        message: Some("425 Can't open data connection\r\n".into()),
                        data: None,
                    };
                }

                // Create new channel entry for data connection
                let mut entry = ChannelEntry::default();
                entry.set_data_socket(Some(data_socket));
                entry.set_data_stream(None);
                entry.set_listener(Some(listener.try_clone().unwrap()));

                // Insert into registry and update client state
                channel_registry.insert(client_addr, entry);
                client.set_data_channel_init(true);

                info!(
                    "Client {} bound to data socket {} in PASV mode",
                    client_addr, data_socket
                );

                // Format PASV reply with socket information
                let response = format!("227 Entering Passive Mode ({})\r\n", data_socket);
                
                info!("Sending PASV response to client {}: {}", client_addr, response.trim());

                return CommandResult {
                    status: CommandStatus::Success,
                    message: Some(response),
                    data: None,
                };
            }
            Err(e) => {
                error!("Failed to bind to {}: {}", data_socket, e);
                return CommandResult {
                    status: CommandStatus::Failure("Port binding failed".into()),
                    message: Some("425 Can't open data connection\r\n".into()),
                    data: None,
                };
            }
        }
    }

    // No ports available in the allowed range
    CommandResult {
        status: CommandStatus::Failure("No available port".into()),
        message: Some("425 Can't open data connection\r\n".into()),
        data: None,
    }
}

/// Handles the PORT command: sets up active FTP mode.
///
/// Parses client-provided address, binds listener, and updates data channel registry.
fn handle_cmd_port(
    client: &mut Client,
    channel_registry: &mut ChannelRegistry,
    addr: &str,
) -> CommandResult {
    let client_addr = *client.client_addr().unwrap();

    // Check authentication
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    // Parse the address string to SocketAddr
    let parsed_addr = match SocketAddr::from_str(addr) {
        Ok(addr) => addr,
        Err(_) => {
            return CommandResult {
                status: CommandStatus::Failure("Invalid address format".into()),
                message: Some("501 Invalid address format. Use IP::PORT\r\n".into()),
                data: None,
            };
        }
    };

    // Validate IP matches client (for security)
    if parsed_addr.ip() != client_addr.ip() {
        return CommandResult {
            status: CommandStatus::Failure("IP mismatch".into()),
            message: Some("501 IP address in PORT must match control connection\r\n".into()),
            data: None,
        };
    }

    // Validate port range
    let port = parsed_addr.port();

    if port < 1024 {
        return CommandResult {
            status: CommandStatus::Failure("Port out of range".into()),
            message: Some("501 Port must be between 1024 and 65535\r\n".into()),
            data: None,
        };
    }

    // Prevent duplicate data channel initialization
    if channel_registry.contains(&client_addr) {
        return CommandResult {
            status: CommandStatus::Failure("Data channel already initialized".into()),
            message: Some("425 Data connection already initialized\r\n".into()),
            data: None,
        };
    }

    // Bind TcpListener on client-specified address
    match std::net::TcpListener::bind(parsed_addr) {
        Ok(listener) => {
            if let Err(e) = listener.set_nonblocking(true) {
                error!("Failed to set non-blocking mode: {}", e);
                return CommandResult {
                    status: CommandStatus::Failure("Failed to configure listener".into()),
                    message: Some("425 Can't open data connection\r\n".into()),
                    data: None,
                };
            }

            let mut entry = ChannelEntry::default();
            entry.set_data_socket(Some(parsed_addr));
            entry.set_data_stream(None);
            entry.set_listener(Some(listener.try_clone().unwrap()));

            channel_registry.insert(client_addr, entry);
            client.set_data_channel_init(true);

            info!(
                "Client {} bound to data socket {} in PORT mode",
                client_addr, parsed_addr
            );

            CommandResult {
                status: CommandStatus::Success,
                message: Some("200 PORT command successful\r\n".into()),
                data: None,
            }
        }
        Err(e) => {
            error!("Failed to bind to {}: {}", parsed_addr, e);
            CommandResult {
                status: CommandStatus::Failure("Port binding failed".into()),
                message: Some("425 Can't open data connection\r\n".into()),
                data: None,
            }
        }
    }
}

/// Handles the custom RAX command: returns a fixed success message.
fn handle_cmd_rax() -> CommandResult {
    CommandResult {
        status: CommandStatus::Success,
        message: Some("200 Rax is the best\r\n".into()),
        data: None,
    }
}

/// Handles unknown or unsupported commands: returns error response.
fn handle_cmd_unknown() -> CommandResult {
    CommandResult {
        status: CommandStatus::Failure("Unknown command".into()),
        message: Some("500 Syntax error, command unrecognized\r\n".into()),
        data: None,
    }
}
