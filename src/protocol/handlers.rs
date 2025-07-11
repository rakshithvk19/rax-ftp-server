//! Command handlers module for the Rax FTP server.
//!
//! This module acts as a thin orchestration layer, delegating business logic
//! to domain-specific modules and translating their results to FTP responses.
//! Updated to support persistent data connections.

use log::info;
use std::io::Write;

use crate::auth;
use crate::client::{self, Client};
use crate::error::AuthError;
use crate::navigate;
use crate::protocol::translators::*;
use crate::protocol::{Command, CommandResult, CommandStatus};
use crate::server::config::ServerConfig;
use crate::storage;
use crate::transfer::{self, ChannelRegistry, setup_data_stream};

/// Dispatches a received FTP command to its corresponding handler.
///
/// Acts as an orchestrator, calling appropriate domain modules and translating
/// their results to FTP protocol responses.
pub fn handle_command(
    client: &mut Client,
    command: &Command,
    channel_registry: &mut ChannelRegistry,
    config: &ServerConfig,
) -> CommandResult {
    match command {
        Command::QUIT => handle_cmd_quit(client, channel_registry),
        Command::USER(username) => handle_cmd_user(client, username),
        Command::PASS(password) => handle_cmd_pass(client, password),
        Command::LIST => handle_cmd_list(client, config, channel_registry),
        Command::PWD => handle_cmd_pwd(client),
        Command::LOGOUT => handle_cmd_logout(client, channel_registry),
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
    // Clean up any persistent data channels for this client
    if let Some(client_addr) = client.client_addr() {
        transfer::cleanup_data_channel(channel_registry, client_addr);
    }

    match client::process_quit(client) {
        Ok(result) => quit_result_to_ftp_response(result),
        Err(error) => client_error_to_ftp_response(error),
    }
}

/// Handles the USER command
fn handle_cmd_user(client: &mut Client, username: &str) -> CommandResult {
    match auth::validate_user(username) {
        Ok(result) => {
            // Update client state based on successful validation
            client.set_user_valid(true);
            client.set_logged_in(false);
            client.set_username(Some(username.to_string()));
            user_result_to_ftp_response(result)
        }
        Err(error) => {
            // Clear client state on validation failure
            client.set_user_valid(false);
            client.set_logged_in(false);
            client.set_username(None);
            auth_error_to_ftp_response(error)
        }
    }
}

/// Handles the PASS command
fn handle_cmd_pass(client: &mut Client, password: &str) -> CommandResult {
    // Check if user was validated first
    if !client.is_user_valid() {
        return auth_error_to_ftp_response(AuthError::InvalidState("Username not provided".into()));
    }

    let username = match client.username() {
        Some(u) => u.clone(),
        None => {
            return auth_error_to_ftp_response(AuthError::InvalidState("Username not set".into()));
        }
    };

    match auth::validate_password(&username, password) {
        Ok(result) => {
            // Update client state for successful login
            client.set_logged_in(true);
            password_result_to_ftp_response(result)
        }
        Err(error) => {
            // Clear login state on failure
            client.set_logged_in(false);
            auth_error_to_ftp_response(error)
        }
    }
}

/// Handles the LIST command
fn handle_cmd_list(
    client: &mut Client,
    config: &ServerConfig,
    channel_registry: &mut ChannelRegistry,
) -> CommandResult {
    // Authentication check
    if !client.is_logged_in() {
        return auth_error_to_ftp_response(AuthError::NotLoggedIn);
    }

    // Data channel check
    if !client.is_data_channel_init() {
        return transfer_error_to_ftp_response(
            crate::error::TransferError::DataChannelNotInitialized,
        );
    }

    // Get client address
    let client_addr = match client.client_addr() {
        Some(addr) => *addr,
        None => {
            return client_error_to_ftp_response(crate::error::ClientError::InvalidState(
                "Client address unknown".into(),
            ));
        }
    };

    // Get directory listing
    let list_result =
        match storage::list_directory(&config.server_root, client.current_virtual_path()) {
            Ok(result) => result,
            Err(error) => return storage_error_to_ftp_response(error),
        };

    // Setup data stream
    let mut data_stream = match setup_data_stream(channel_registry, &client_addr) {
        Some(stream) => stream,
        None => {
            return transfer_error_to_ftp_response(
                crate::error::TransferError::DataChannelSetupFailed(
                    "Failed to establish data connection".into(),
                ),
            );
        }
    };

    // Send the listing over data connection
    let listing_data = list_result.entries.join("\r\n") + "\r\n";
    match data_stream.write_all(listing_data.as_bytes()) {
        Ok(_) => match data_stream.flush() {
            Ok(_) => {
                info!(
                    "Directory listing sent successfully to client {}",
                    client_addr
                );

                // Clean up only the data stream, keep persistent setup
                transfer::cleanup_data_stream_only(channel_registry, &client_addr);

                CommandResult {
                    status: CommandStatus::Success,
                    message: Some("226 Directory send OK\r\n".into()),
                    //
                }
            }
            Err(e) => {
                // Clean up only the data stream on error
                transfer::cleanup_data_stream_only(channel_registry, &client_addr);

                transfer_error_to_ftp_response(crate::error::TransferError::TransferFailed(e))
            }
        },
        Err(e) => {
            // Clean up only the data stream on error
            transfer::cleanup_data_stream_only(channel_registry, &client_addr);
            transfer_error_to_ftp_response(crate::error::TransferError::TransferFailed(e))
        }
    }
}

/// Handles the PWD command
fn handle_cmd_pwd(client: &Client) -> CommandResult {
    // Authentication check
    if !client.is_logged_in() {
        return auth_error_to_ftp_response(AuthError::NotLoggedIn);
    }

    match navigate::get_working_directory(client.current_virtual_path()) {
        Ok(result) => pwd_result_to_ftp_response(result),
        Err(error) => navigate_error_to_ftp_response(error),
    }
}

/// Handles the LOGOUT command
fn handle_cmd_logout(client: &mut Client, channel_registry: &mut ChannelRegistry) -> CommandResult {
    // Clean up any persistent data channels for this client
    if let Some(client_addr) = client.client_addr() {
        transfer::cleanup_data_channel(channel_registry, client_addr);
    }

    match client::process_logout(client) {
        Ok(result) => logout_result_to_ftp_response(result),
        Err(error) => client_error_to_ftp_response(error),
    }
}

/// Handles the RETR command
fn handle_cmd_retr(
    client: &mut Client,
    filename: &str,
    channel_registry: &mut ChannelRegistry,
    config: &ServerConfig,
) -> CommandResult {
    // Authentication check
    if !client.is_logged_in() {
        return auth_error_to_ftp_response(AuthError::NotLoggedIn);
    }

    // Data channel check
    if !client.is_data_channel_init() {
        return transfer_error_to_ftp_response(
            crate::error::TransferError::DataChannelNotInitialized,
        );
    }

    // Prepare file retrieval
    let retrieve_result = match storage::prepare_file_retrieval(
        &config.server_root,
        client.current_virtual_path(),
        filename,
    ) {
        Ok(result) => result,
        Err(error) => return storage_error_to_ftp_response(error),
    };

    // Get client address
    let client_addr = match client.client_addr() {
        Some(addr) => *addr,
        None => {
            return client_error_to_ftp_response(crate::error::ClientError::InvalidState(
                "Client address unknown".into(),
            ));
        }
    };

    info!(
        "Client {} requested to retrieve {} (virtual: {}, real: {})",
        client_addr,
        filename,
        retrieve_result.virtual_path,
        retrieve_result.file_path.display()
    );

    // Setup data stream
    let data_stream = match setup_data_stream(channel_registry, &client_addr) {
        Some(stream) => stream,
        None => {
            return transfer_error_to_ftp_response(
                crate::error::TransferError::DataChannelSetupFailed(
                    "Failed to establish data connection".into(),
                ),
            );
        }
    };

    // Delegate file download to transfer module
    let result = match crate::transfer::handle_file_download(
        data_stream,
        &retrieve_result.file_path.to_string_lossy(),
    ) {
        Ok((status, msg)) => {
            // Clean up only the data stream, keep persistent setup
            transfer::cleanup_data_stream_only(channel_registry, &client_addr);
            
            CommandResult {
                status,
                message: Some(msg.into()),
                //
            }
        }
        Err((status, msg)) => {
            // Clean up only the data stream on error
            transfer::cleanup_data_stream_only(channel_registry, &client_addr);
            
            CommandResult {
                status,
                message: Some(msg.into()),
                //
            }
        }
    };

    result
}

/// Handles the STOR command
fn handle_cmd_stor(
    client: &mut Client,
    filename: &str,
    channel_registry: &mut ChannelRegistry,
    config: &ServerConfig,
) -> CommandResult {
    // Authentication check
    if !client.is_logged_in() {
        return auth_error_to_ftp_response(AuthError::NotLoggedIn);
    }

    // Data channel check
    if !client.is_data_channel_init() {
        return transfer_error_to_ftp_response(
            crate::error::TransferError::DataChannelNotInitialized,
        );
    }

    // Prepare file storage
    let store_result = match storage::prepare_file_storage(
        &config.server_root,
        client.current_virtual_path(),
        filename,
    ) {
        Ok(result) => result,
        Err(error) => return storage_error_to_ftp_response(error),
    };

    // Get client address
    let client_addr = match client.client_addr() {
        Some(addr) => *addr,
        None => {
            return client_error_to_ftp_response(crate::error::ClientError::InvalidState(
                "Client address unknown".into(),
            ));
        }
    };

    info!(
        "Client {} requested to store {} (virtual: {}, real: {})",
        client_addr,
        filename,
        store_result.virtual_path,
        store_result.file_path.display()
    );

    // Setup data stream
    let data_stream = match setup_data_stream(channel_registry, &client_addr) {
        Some(stream) => stream,
        None => {
            return transfer_error_to_ftp_response(
                crate::error::TransferError::DataChannelSetupFailed(
                    "Failed to establish data connection".into(),
                ),
            );
        }
    };

    // Delegate file upload to transfer module
    let result = match crate::transfer::handle_file_upload(
        data_stream,
        &store_result.file_path.to_string_lossy(),
        &store_result.temp_path.to_string_lossy(),
    ) {
        Ok((status, msg)) => {
            // Clean up only the data stream, keep persistent setup
            transfer::cleanup_data_stream_only(channel_registry, &client_addr);
            
            CommandResult {
                status,
                message: Some(msg.into()),
                //
            }
        }
        Err((status, msg)) => {
            // Clean up only the data stream on error
            transfer::cleanup_data_stream_only(channel_registry, &client_addr);
            
            CommandResult {
                status,
                message: Some(msg.into()),
                //
            }
        }
    };

    result
}

/// Handles the DEL command
fn handle_cmd_del(client: &Client, filename: &str, config: &ServerConfig) -> CommandResult {
    // Authentication check
    if !client.is_logged_in() {
        return auth_error_to_ftp_response(AuthError::NotLoggedIn);
    }

    // Delete file
    match storage::delete_file(&config.server_root, client.current_virtual_path(), filename) {
        Ok(result) => {
            info!(
                "Client {} deleted file {} (virtual: {}, real: {})",
                client
                    .client_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                filename,
                result.virtual_path,
                result.file_path.display()
            );
            delete_result_to_ftp_response(result)
        }
        Err(error) => storage_error_to_ftp_response(error),
    }
}

/// Handles the CWD command
fn handle_cmd_cwd(client: &mut Client, path: &str, config: &ServerConfig) -> CommandResult {
    // Authentication check
    if !client.is_logged_in() {
        return auth_error_to_ftp_response(AuthError::NotLoggedIn);
    }

    // Change directory
    match navigate::change_directory(&config.server_root, client.current_virtual_path(), path) {
        Ok(result) => {
            // Update client's virtual path
            client.set_current_virtual_path(result.new_virtual_path.clone());

            info!(
                "Client {} changed directory to {} (real: {})",
                client
                    .client_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                result.new_virtual_path,
                result.real_path.display()
            );

            cwd_result_to_ftp_response(result)
        }
        Err(error) => navigate_error_to_ftp_response(error),
    }
}

/// Handles the PASV command
fn handle_cmd_pasv(client: &mut Client, channel_registry: &mut ChannelRegistry) -> CommandResult {
    // Authentication check
    if !client.is_logged_in() {
        return auth_error_to_ftp_response(AuthError::NotLoggedIn);
    }

    let client_addr = match client.client_addr() {
        Some(addr) => *addr,
        None => {
            return client_error_to_ftp_response(crate::error::ClientError::InvalidState(
                "Client address unknown".into(),
            ));
        }
    };

    // Setup passive mode (this will replace any existing setup)
    match transfer::setup_passive_mode(channel_registry, client_addr) {
        Ok(result) => {
            client.set_data_channel_init(true);
            info!(
                "Sending PASV response to client {}: 227 Entering Passive Mode ({})",
                client_addr, result.data_socket
            );
            passive_result_to_ftp_response(result)
        }
        Err(error) => transfer_error_to_ftp_response(error),
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
        return auth_error_to_ftp_response(AuthError::NotLoggedIn);
    }

    let client_addr = match client.client_addr() {
        Some(addr) => *addr,
        None => {
            return client_error_to_ftp_response(crate::error::ClientError::InvalidState(
                "Client address unknown".into(),
            ));
        }
    };

    // Setup active mode (this will replace any existing setup)
    match transfer::setup_active_mode(channel_registry, client_addr, addr) {
        Ok(result) => {
            client.set_data_channel_init(true);
            active_result_to_ftp_response(result)
        }
        Err(error) => transfer_error_to_ftp_response(error),
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
