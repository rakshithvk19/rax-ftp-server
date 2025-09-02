//! Command handlers module for the Rax FTP server.
//!
//! This module acts as a thin orchestration layer, delegating business logic
//! to domain-specific modules and translating their results to FTP responses.
//! Updated to support persistent data connections.

use log::info;
use std::future::Future;
use std::pin::Pin;

use crate::auth;
use crate::client::Client;
use crate::error::AuthError;
use crate::error::TransferError;
use crate::navigate;
use crate::protocol::{Command, CommandResult, CommandStatus};
use crate::server::config::ServerConfig;
use crate::storage;
use crate::transfer::{
    self, receive_file_upload, send_directory_listing, setup_data_stream,
    validate_client_and_data_channel, ChannelRegistry,
};

/// Dispatches a received FTP command to its corresponding handler.
///
/// Acts as an orchestrator, calling appropriate domain modules and translating
/// their results to FTP protocol responses.
pub async fn handle_command<F>(
    client: &mut Client,
    command: &Command,
    channel_registry: &mut ChannelRegistry,
    config: &ServerConfig,
    send_intermediate: &F,
) -> CommandResult
where
    F: Fn(&str) -> Pin<Box<dyn Future<Output = Result<(), std::io::Error>> + Send>>,
{
    match command {
        Command::QUIT => handle_cmd_quit(client, channel_registry),
        Command::USER(username) => handle_cmd_user(client, username),
        Command::PASS(password) => handle_cmd_pass(client, password),
        Command::LIST => handle_cmd_list(client, config, channel_registry, send_intermediate).await,
        Command::PWD => handle_cmd_pwd(client),
        Command::LOGOUT => handle_cmd_logout(client, channel_registry),
        Command::RETR(filename) => {
            handle_cmd_retr(
                client,
                filename,
                channel_registry,
                config,
                send_intermediate,
            )
            .await
        }
        Command::STOR(filename) => {
            handle_cmd_stor(
                client,
                filename,
                channel_registry,
                config,
                send_intermediate,
            )
            .await
        }
        Command::DEL(filename) => handle_cmd_del(client, filename, config),
        Command::CWD(path) => handle_cmd_cwd(client, path, config),
        Command::PASV => handle_cmd_pasv(client, channel_registry),
        Command::PORT(addr) => handle_cmd_port(client, channel_registry, addr),
        Command::RAX => handle_cmd_rax(),
        Command::UNKNOWN => handle_cmd_unknown(),
    }
}

/// Handles authentication commands during the login phase
pub fn handle_auth_command(client: &mut Client, command: &Command) -> CommandResult {
    match command {
        Command::USER(username) => handle_cmd_user(client, username),
        Command::PASS(password) => handle_cmd_pass(client, password),
        _ => CommandResult {
            status: CommandStatus::Failure("Authentication required".into()),
            message: Some("530 Please login with USER and PASS\r\n".into()),
            //
        },
    }
}

/// Handles the QUIT command
fn handle_cmd_quit(client: &mut Client, channel_registry: &mut ChannelRegistry) -> CommandResult {
    let client_addr_str = client
        .client_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    info!("Processing QUIT command for client {}", client_addr_str);

    // Clean up any persistent data channels for this client
    if let Some(client_addr) = client.client_addr() {
        info!(
            "Cleaning up data channels for quitting client {}",
            client_addr
        );
        transfer::cleanup_data_channel(channel_registry, client_addr);
    }

    // Logout the client directly
    client.logout();

    info!("Client {} has quit successfully", client_addr_str);

    CommandResult {
        status: CommandStatus::CloseConnection,
        message: Some("221 Goodbye\r\n".into()),
    }
}

/// Handles the USER command
fn handle_cmd_user(client: &mut Client, username: &str) -> CommandResult {
    match auth::validate_user(username) {
        Ok(_) => {
            // Update client state based on successful validation
            client.set_user_valid(true);
            client.set_logged_in(false);
            client.set_username(Some(username.to_string()));
            CommandResult {
                status: CommandStatus::Success,
                message: Some("331 Password required\r\n".into()),
            }
        }
        Err(error) => {
            // Clear client state on validation failure
            client.set_user_valid(false);
            client.set_logged_in(false);
            client.set_username(None);

            let (code, message) = match error {
                AuthError::InvalidUsername(u) => (530, format!("Invalid username: {}", u)),
                AuthError::UserNotFound(u) => (530, format!("Unknown user '{}'", u)),
                AuthError::MalformedInput(_) => (530, "Malformed input".to_string()),
                _ => (530, "Authentication error".to_string()),
            };

            CommandResult {
                status: CommandStatus::Failure(message.clone()),
                message: Some(format!("{} {}\r\n", code, message)),
            }
        }
    }
}

/// Handles the PASS command
fn handle_cmd_pass(client: &mut Client, password: &str) -> CommandResult {
    // Check if user was validated first
    if !client.is_user_valid() {
        return CommandResult {
            status: CommandStatus::Failure("Username not provided".into()),
            message: Some("530 Username not provided\r\n".into()),
        };
    }

    let username = match client.username() {
        Some(u) => u.clone(),
        None => {
            return CommandResult {
                status: CommandStatus::Failure("Username not set".into()),
                message: Some("530 Username not set\r\n".into()),
            };
        }
    };

    match auth::validate_password(&username, password) {
        Ok(_) => {
            // Update client state for successful login
            client.set_logged_in(true);
            CommandResult {
                status: CommandStatus::Success,
                message: Some("230 Login successful\r\n".into()),
            }
        }
        Err(error) => {
            // Clear login state on failure
            client.set_logged_in(false);

            let (code, message) = match error {
                AuthError::InvalidPassword(u) => (530, format!("Invalid password for user: {}", u)),
                AuthError::UserNotFound(u) => (530, format!("Unknown user '{}'", u)),
                AuthError::MalformedInput(_) => (530, "Malformed input".to_string()),
                _ => (530, "Authentication failed".to_string()),
            };

            CommandResult {
                status: CommandStatus::Failure(message.clone()),
                message: Some(format!("{} {}\r\n", code, message)),
            }
        }
    }
}

async fn handle_cmd_list<F>(
    client: &mut Client,
    config: &ServerConfig,
    channel_registry: &mut ChannelRegistry,
    send_intermediate: &F, // For sending 150 immediately
) -> CommandResult
// Still return CommandResult!
where
    F: Fn(&str) -> Pin<Box<dyn Future<Output = Result<(), std::io::Error>> + Send>>,
{
    // Authentication and data channel validation
    if !validate_client_and_data_channel(client) {
        if !client.is_logged_in() {
            return CommandResult {
                status: CommandStatus::Failure("Not logged in".into()),
                message: Some("530 Not logged in\r\n".into()),
            };
        }
        return CommandResult {
            status: CommandStatus::Failure("Data channel not initialized".into()),
            message: Some("425 Data channel not initialized\r\n".into()),
        };
    }

    // 1. Send 150 IMMEDIATELY via callback
    if let Err(_) =
        send_intermediate("150 Opening ASCII mode data connection for file list\r\n").await
    {
        return CommandResult {
            status: CommandStatus::Failure("Send failed".into()),
            message: Some("421 Service not available\r\n".into()),
        };
    }

    // Get client address
    let client_addr = match client.client_addr() {
        Some(addr) => *addr,
        None => {
            return CommandResult {
                status: CommandStatus::Failure("Client address unknown".into()),
                message: Some("530 Client address unknown\r\n".into()),
            };
        }
    };

    // Get directory listing
    let entries = match storage::list_directory(&config.server_root, client.current_virtual_path())
    {
        Ok(entries) => entries,
        Err(error) => {
            let (code, message) = match error {
                crate::error::StorageError::DirectoryNotFound(p) => {
                    (550, format!("{}: Directory not found", p))
                }
                crate::error::StorageError::PermissionDenied(p) => {
                    (550, format!("{}: Permission denied", p))
                }
                crate::error::StorageError::IoError(e) => (550, format!("I/O error: {}", e)),
                _ => (550, "Directory listing failed".to_string()),
            };
            return CommandResult {
                status: CommandStatus::Failure(message.clone()),
                message: Some(format!("{} {}\r\n", code, message)),
            };
        }
    };

    // Send directory listing over data channel
    match send_directory_listing(channel_registry, &client_addr, entries) {
        Ok(_) => {
            // Clean up the stream but keep persistent setup
            transfer::cleanup_data_stream_only(channel_registry, &client_addr);

            CommandResult {
                status: CommandStatus::Success,
                message: Some("226 Directory send OK\r\n".into()),
            }
        }
        Err(_) => {
            transfer::cleanup_data_stream_only(channel_registry, &client_addr);
            CommandResult {
                status: CommandStatus::Failure("Transfer failed".into()),
                message: Some("426 Transfer failed\r\n".into()),
            }
        }
    }
}
/// Handles the PWD command
fn handle_cmd_pwd(client: &Client) -> CommandResult {
    // Authentication check
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
        };
    }

    CommandResult {
        status: CommandStatus::Success,
        message: Some(format!("257 \"{}\"\r\n", client.current_virtual_path())),
    }
}

/// Handles the LOGOUT command
fn handle_cmd_logout(client: &mut Client, channel_registry: &mut ChannelRegistry) -> CommandResult {
    let client_addr_str = client
        .client_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    info!("Processing LOGOUT command for client {}", client_addr_str);

    // Check if user is actually logged in
    if !client.is_logged_in() {
        info!(
            "LOGOUT attempted by client {} who is not logged in",
            client_addr_str
        );
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 User not logged in\r\n".into()),
        };
    }

    // Clean up any persistent data channels for this client
    if let Some(client_addr) = client.client_addr() {
        info!(
            "Cleaning up data channels for logging out client {}",
            client_addr
        );
        transfer::cleanup_data_channel(channel_registry, client_addr);
    }

    // Logout the client directly
    client.logout();

    info!("Client {} has logged out successfully", client_addr_str);

    CommandResult {
        status: CommandStatus::Success,
        message: Some("221 Logout successful\r\n".into()),
    }
}

/// Handles the RETR command
async fn handle_cmd_retr<F>(
    client: &mut Client,
    filename: &str,
    channel_registry: &mut ChannelRegistry,
    config: &ServerConfig,
    send_intermediate: &F,
) -> CommandResult
where
    F: Fn(&str) -> Pin<Box<dyn Future<Output = Result<(), std::io::Error>> + Send>>,
{
    // Authentication and data channel validation
    if !validate_client_and_data_channel(client) {
        if !client.is_logged_in() {
            return CommandResult {
                status: CommandStatus::Failure("Not logged in".into()),
                message: Some("530 Not logged in\r\n".into()),
            };
        }
        return CommandResult {
            status: CommandStatus::Failure("Data channel not initialized".into()),
            message: Some("425 Data channel not initialized\r\n".into()),
        };
    }

    // 1. Send 150 IMMEDIATELY via callback
    if let Err(_) =
        send_intermediate("150 Opening BINARY mode data connection for file transfer\r\n").await
    {
        return CommandResult {
            status: CommandStatus::Failure("Send failed".into()),
            message: Some("421 Service not available\r\n".into()),
        };
    }

    // Prepare file retrieval
    let file_path = match storage::prepare_file_retrieval(
        &config.server_root,
        client.current_virtual_path(),
        filename,
    ) {
        Ok(path) => path,
        Err(error) => {
            let (code, message) = match error {
                crate::error::StorageError::FileNotFound(p) => {
                    (550, format!("{}: File not found", p))
                }
                crate::error::StorageError::PermissionDenied(p) => {
                    (550, format!("{}: Permission denied", p))
                }
                crate::error::StorageError::IoError(e) => (550, format!("I/O error: {}", e)),
                _ => (550, "File retrieval failed".to_string()),
            };
            return CommandResult {
                status: CommandStatus::Failure(message.clone()),
                message: Some(format!("{} {}\r\n", code, message)),
            };
        }
    };

    // Get client address
    let client_addr = match client.client_addr() {
        Some(addr) => *addr,
        None => {
            return CommandResult {
                status: CommandStatus::Failure("Client address unknown".into()),
                message: Some("530 Client address unknown\r\n".into()),
            };
        }
    };

    info!(
        "Client {} requested to retrieve {} (real: {})",
        client_addr,
        filename,
        file_path.display()
    );

    // Setup data stream and perform file download
    let data_stream = match setup_data_stream(channel_registry, &client_addr) {
        Some(stream) => stream,
        None => {
            return CommandResult {
                status: CommandStatus::Failure("Failed to establish data connection".into()),
                message: Some("425 Failed to establish data connection\r\n".into()),
            };
        }
    };

    // Delegate file download to transfer module
    match crate::transfer::handle_file_download(data_stream, &file_path.to_string_lossy()) {
        Ok((status, _)) => {
            // Clean up only the data stream, keep persistent setup
            transfer::cleanup_data_stream_only(channel_registry, &client_addr);

            CommandResult {
                status,
                message: Some("226 Transfer complete\r\n".into()),
            }
        }
        Err((status, _)) => {
            // Clean up only the data stream on error
            transfer::cleanup_data_stream_only(channel_registry, &client_addr);

            CommandResult {
                status,
                message: Some("426 Transfer failed\r\n".into()),
            }
        }
    }
}

/// Handles the STOR command
async fn handle_cmd_stor<F>(
    client: &mut Client,
    filename: &str,
    channel_registry: &mut ChannelRegistry,
    config: &ServerConfig,
    send_intermediate: &F,
) -> CommandResult
where
    F: Fn(&str) -> Pin<Box<dyn Future<Output = Result<(), std::io::Error>> + Send>>,
{
    // Authentication and data channel validation
    if !validate_client_and_data_channel(client) {
        if !client.is_logged_in() {
            return CommandResult {
                status: CommandStatus::Failure("Not logged in".into()),
                message: Some("530 Not logged in\r\n".into()),
            };
        }
        return CommandResult {
            status: CommandStatus::Failure("Data channel not initialized".into()),
            message: Some("425 Data channel not initialized\r\n".into()),
        };
    }

    // 1. Send 150 IMMEDIATELY via callback
    if let Err(_) =
        send_intermediate("150 Opening BINARY mode data connection for file transfer\r\n").await
    {
        return CommandResult {
            status: CommandStatus::Failure("Send failed".into()),
            message: Some("421 Service not available\r\n".into()),
        };
    }

    // Prepare file storage
    let (file_path, temp_path) = match storage::prepare_file_storage(
        &config.server_root,
        client.current_virtual_path(),
        filename,
    ) {
        Ok((file_path, temp_path)) => (file_path, temp_path),
        Err(error) => {
            let (code, message) = match error {
                crate::error::StorageError::FileAlreadyExists(p) => {
                    (550, format!("{}: File already exists", p))
                }
                crate::error::StorageError::PermissionDenied(p) => {
                    (550, format!("{}: Permission denied", p))
                }
                crate::error::StorageError::UploadInProgress(p) => {
                    (550, format!("{}: Upload already in progress", p))
                }
                crate::error::StorageError::IoError(e) => (550, format!("I/O error: {}", e)),
                _ => (550, "File storage preparation failed".to_string()),
            };
            return CommandResult {
                status: CommandStatus::Failure(message.clone()),
                message: Some(format!("{} {}\r\n", code, message)),
            };
        }
    };

    // Get client address
    let client_addr = match client.client_addr() {
        Some(addr) => *addr,
        None => {
            return CommandResult {
                status: CommandStatus::Failure("Client address unknown".into()),
                message: Some("530 Client address unknown\r\n".into()),
            };
        }
    };

    info!(
        "Client {} requested to store {} (real: {})",
        client_addr,
        filename,
        file_path.display()
    );

    // Receive file upload over data channel
    match receive_file_upload(
        channel_registry,
        &client_addr,
        &file_path.to_string_lossy(),
        &temp_path.to_string_lossy(),
    ) {
        Ok(_) => {
            // Clean up the stream but keep persistent setup
            transfer::cleanup_data_stream_only(channel_registry, &client_addr);

            CommandResult {
                status: CommandStatus::Success,
                message: Some("226 Transfer complete\r\n".into()),
            }
        }
        Err(_) => {
            transfer::cleanup_data_stream_only(channel_registry, &client_addr);
            CommandResult {
                status: CommandStatus::Failure("Transfer failed".into()),
                message: Some("426 Transfer failed\r\n".into()),
            }
        }
    }
}

/// Handles the DEL command
fn handle_cmd_del(client: &Client, filename: &str, config: &ServerConfig) -> CommandResult {
    // Authentication check
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
        };
    }

    // Delete file
    match storage::delete_file(&config.server_root, client.current_virtual_path(), filename) {
        Ok(_) => {
            info!(
                "Client {} deleted file {}",
                client
                    .client_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                filename
            );
            CommandResult {
                status: CommandStatus::Success,
                message: Some("250 File deleted successfully\r\n".into()),
            }
        }
        Err(error) => {
            let (code, message) = match error {
                crate::error::StorageError::FileNotFound(p) => {
                    (550, format!("{}: File not found", p))
                }
                crate::error::StorageError::PermissionDenied(p) => {
                    (550, format!("{}: Permission denied", p))
                }
                crate::error::StorageError::IoError(e) => (550, format!("I/O error: {}", e)),
                _ => (550, "File deletion failed".to_string()),
            };
            CommandResult {
                status: CommandStatus::Failure(message.clone()),
                message: Some(format!("{} {}\r\n", code, message)),
            }
        }
    }
}

/// Handles the CWD command
fn handle_cmd_cwd(client: &mut Client, path: &str, config: &ServerConfig) -> CommandResult {
    // Authentication check
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
        };
    }

    // Change directory
    match navigate::change_directory(&config.server_root, client.current_virtual_path(), path) {
        Ok(new_virtual_path) => {
            // Update client's virtual path
            client.set_current_virtual_path(new_virtual_path.clone());

            info!(
                "Client {} changed directory to {}",
                client
                    .client_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                new_virtual_path
            );

            CommandResult {
                status: CommandStatus::Success,
                message: Some("250 Directory changed successfully\r\n".into()),
            }
        }
        Err(error) => {
            let (code, message) = match error {
                crate::error::NavigateError::DirectoryNotFound(p) => {
                    (550, format!("{}: Directory not found", p))
                }
                crate::error::NavigateError::NotADirectory(p) => {
                    (550, format!("{}: Not a directory", p))
                }
                crate::error::NavigateError::PermissionDenied(p) => {
                    (550, format!("{}: Permission denied", p))
                }
                crate::error::NavigateError::PathTraversal(p) => {
                    (550, format!("Path traversal attempt: {}", p))
                }
                _ => (550, "Directory change failed".to_string()),
            };
            CommandResult {
                status: CommandStatus::Failure(message.clone()),
                message: Some(format!("{} {}\r\n", code, message)),
            }
        }
    }
}

/// Handles the PASV command
fn handle_cmd_pasv(client: &mut Client, channel_registry: &mut ChannelRegistry) -> CommandResult {
    // Authentication check
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
        };
    }

    let client_addr = match client.client_addr() {
        Some(addr) => *addr,
        None => {
            return CommandResult {
                status: CommandStatus::Failure("Client address unknown".into()),
                message: Some("530 Client address unknown\r\n".into()),
            };
        }
    };

    // Setup passive mode (this will replace any existing setup)
    match transfer::setup_passive_mode(channel_registry, client_addr) {
        Ok(data_socket) => {
            client.set_data_channel_init(true);
            info!(
                "Sending PASV response to client {}: 227 Entering Passive Mode ({})",
                client_addr, data_socket
            );
            CommandResult {
                status: CommandStatus::Success,
                message: Some(format!("227 Entering Passive Mode ({})\r\n", data_socket)),
            }
        }
        Err(error) => {
            let (code, message) = match error {
                TransferError::NoAvailablePort => (425, "No available port".to_string()),
                TransferError::PortBindingFailed(addr, e) => {
                    (425, format!("Can't bind to {}: {}", addr, e))
                }
                TransferError::ListenerConfigurationFailed(e) => {
                    (425, format!("Listener config failed: {}", e))
                }
                _ => (425, "Passive mode setup failed".to_string()),
            };
            CommandResult {
                status: CommandStatus::Failure(message.clone()),
                message: Some(format!("{} {}\r\n", code, message)),
            }
        }
    }
}

/// Handles the PORT command
fn handle_cmd_port(
    client: &mut Client,
    channel_registry: &mut ChannelRegistry,
    addr: &str,
) -> CommandResult {
    // Authentication check
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
        };
    }

    let client_addr = match client.client_addr() {
        Some(addr) => *addr,
        None => {
            return CommandResult {
                status: CommandStatus::Failure("Client address unknown".into()),
                message: Some("530 Client address unknown\r\n".into()),
            };
        }
    };

    // Setup active mode (this will replace any existing setup)
    match transfer::setup_active_mode(channel_registry, client_addr, addr) {
        Ok(_) => {
            client.set_data_channel_init(true);
            CommandResult {
                status: CommandStatus::Success,
                message: Some("200 PORT command successful\r\n".into()),
            }
        }
        Err(error) => {
            let (code, message) = match error {
                TransferError::InvalidPortCommand(msg) => (501, msg),
                TransferError::IpMismatch { expected, provided } => (
                    501,
                    format!("IP mismatch: expected {}, got {}", expected, provided),
                ),
                TransferError::InvalidPortRange(port) => (
                    501,
                    format!("Port {} out of range (must be 1024-65535)", port),
                ),
                _ => (425, "Active mode setup failed".to_string()),
            };
            CommandResult {
                status: CommandStatus::Failure(message.clone()),
                message: Some(format!("{} {}\r\n", code, message)),
            }
        }
    }
}

/// Handles the custom RAX command
fn handle_cmd_rax() -> CommandResult {
    CommandResult {
        status: CommandStatus::Success,
        message: Some("200 Rax is the best\r\n".into()),
        //
    }
}

/// Handles unknown or unsupported commands
fn handle_cmd_unknown() -> CommandResult {
    CommandResult {
        status: CommandStatus::Failure("Unknown command".into()),
        message: Some("500 Syntax error, command unrecognized\r\n".into()),
        //
    }
}
