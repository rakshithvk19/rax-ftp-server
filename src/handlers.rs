//! Command handlers module for the Rax FTP server.
//!
//! This module defines handler functions for FTP commands, handling
//! authentication, file operations, directory management, and data channel
//! setup per client connection.

use crate::auth;
use crate::channel_registry::{ChannelEntry, ChannelRegistry};
use crate::client::Client;
use crate::command::{Command, CommandData, CommandResult, CommandStatus};
use crate::data_channel::setup_data_stream;
use crate::file_transfer::{handle_file_download, handle_file_upload};
use log::{error, info};

use std::env;
use std::fs;
use std::net::{SocketAddr, TcpListener};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

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
) -> CommandResult {
    match command {
        Command::QUIT => handle_cmd_quit(client),
        Command::USER(username) => handle_cmd_user(client, username),
        Command::PASS(password) => handle_cmd_pass(client, password),
        Command::LIST => handle_cmd_list(client),
        Command::PWD => handle_cmd_pwd(client),
        Command::LOGOUT => handle_cmd_logout(client),
        Command::RETR(filename) => handle_cmd_retr(client, filename, channel_registry),
        Command::STOR(filename) => handle_cmd_stor(client, filename, channel_registry),
        Command::DEL(filename) => handle_cmd_del(client, filename),
        Command::CWD(path) => handle_cmd_cwd(client, path),
        Command::PASV => handle_cmd_pasv(client, channel_registry),
        Command::PORT(addr) => handle_cmd_port(client, channel_registry, addr),
        Command::RAX => handle_cmd_rax(),
        Command::UNKNOWN => handle_cmd_unknown(),
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
/// Reads the `./test_dir` directory and returns file names in the response data.
fn handle_cmd_list(client: &Client) -> CommandResult {
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    let retries = 3;
    for attempt in 1..=retries {
        match fs::read_dir("./test_dir") {
            Ok(entries) => {
                let mut file_list = vec![];
                for entry in entries.flatten() {
                    file_list.push(entry.file_name().to_string_lossy().to_string());
                }
                return CommandResult {
                    status: CommandStatus::Success,
                    message: Some("226 Directory listing successful\r\n".into()),
                    data: Some(CommandData::DirectoryListing(file_list)),
                };
            }
            Err(e) => {
                if attempt < retries && e.kind() == std::io::ErrorKind::PermissionDenied {
                    thread::sleep(Duration::from_millis(100 * attempt as u64));
                    continue;
                } else {
                    error!("Failed to list directory: {}", e);
                    return CommandResult {
                        status: CommandStatus::Failure(e.to_string()),
                        message: Some("550 Failed to list directory\r\n".into()),
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

    // 5. Check if file already exists
    if fs::metadata(filename).is_ok() {
        return CommandResult {
            status: CommandStatus::Failure("File exists".into()),
            message: Some("550 File already exists\r\n".into()),
            data: None,
        };
    }

    // 6. Retrieve client address for logging
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
        "Client {} requested to store data for {}",
        client_addr, filename
    );

    // 7. Setup data stream for file upload
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

    // 8. Delegate file upload to file transfer module
    match handle_file_upload(data_stream, filename) {
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

    // 5. Check if file exists
    if fs::metadata(filename).is_err() {
        return CommandResult {
            status: CommandStatus::Failure("File not found".into()),
            message: Some("550 File not found\r\n".into()),
            data: None,
        };
    }

    // 6. Retrieve client address
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
        "Client {} requested to retrieve data for {}",
        client_addr, filename
    );

    // 7. Setup data stream for file download
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

    // 8. Delegate file download to file transfer module
    match handle_file_download(data_stream, filename) {
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
fn handle_cmd_del(client: &mut Client, filename: &str) -> CommandResult {
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

    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return CommandResult {
            status: CommandStatus::Failure("Invalid filename".into()),
            message: Some("550 Invalid filename\r\n".into()),
            data: None,
        };
    }

    let retries = 3;
    for attempt in 1..=retries {
        match fs::remove_file(filename) {
            Ok(_) => {
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
                    error!("Failed to delete file '{}': {}", filename, e);
                    return CommandResult {
                        status: CommandStatus::Failure(e.to_string()),
                        message: Some("550 Failed to delete file\r\n".into()),
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

/// Handles the CWD command: changes the current working directory of the server.
///
/// Returns success if directory changed; failure otherwise.
fn handle_cmd_cwd(client: &Client, path: &str) -> CommandResult {
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

    let retries = 3;
    for attempt in 1..=retries {
        match env::set_current_dir(path) {
            Ok(_) => {
                return CommandResult {
                    status: CommandStatus::Success,
                    message: Some("250 Directory changed successfully\r\n".into()),
                    data: None,
                };
            }
            Err(e) => {
                if attempt < retries && e.kind() == std::io::ErrorKind::PermissionDenied {
                    thread::sleep(Duration::from_millis(100 * attempt as u64));
                    continue;
                } else {
                    error!("Failed to change directory to '{}': {}", path, e);
                    return CommandResult {
                        status: CommandStatus::Failure(e.to_string()),
                        message: Some("550 Failed to change directory\r\n".into()),
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

/// Handles the PWD command: returns the current working directory to the client.
///
/// Returns the directory path on success; error message otherwise.
fn handle_cmd_pwd(client: &Client) -> CommandResult {
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    match env::current_dir() {
        Ok(path) => CommandResult {
            status: CommandStatus::Success,
            message: Some(format!("257 \"{}\"\r\n", path.display())),
            data: None,
        },
        Err(e) => {
            error!("Failed to get current directory: {}", e);
            CommandResult {
                status: CommandStatus::Failure(e.to_string()),
                message: Some("550 Failed to get current directory\r\n".into()),
                data: None,
            }
        }
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
