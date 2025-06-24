// handlers.rs
// This file defines handlers for FTP commands, coordinating authentication,
// file operations, directory management, and data channel setup for each client.

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

// Handle a single command and update server
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
        Command::RETR(filename) => handle_cmd_retr(client, &filename, channel_registry),
        Command::STOR(filename) => handle_cmd_stor(client, &filename, channel_registry),
        Command::DEL(filename) => handle_cmd_del(client, &filename),
        Command::CWD(path) => handle_cmd_cwd(client, &path),
        Command::PASV() => handle_cmd_pasv(client, channel_registry),
        Command::PORT(addr) => handle_cmd_port(client, channel_registry, &addr),
        Command::RAX => handle_cmd_rax(),
        Command::UNKNOWN => handle_cmd_unknown(),
    }
}

fn handle_cmd_quit(client: &mut Client) -> CommandResult {
    client.logout();

    CommandResult {
        status: CommandStatus::CloseConnection,
        message: Some("221 Goodbye\r\n".into()),
        data: None,
    }
}

fn handle_cmd_user(client: &mut Client, username: &String) -> CommandResult {
    match auth::validate_user(&username) {
        Ok(response) => {
            client.set_user_valid(true);
            client.set_logged_in(false);
            client.set_username(Some(username.clone()));
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

fn handle_cmd_pass(client: &mut Client, password: &String) -> CommandResult {
    if client.is_user_valid() {
        if let Some(username) = &client.username() {
            match auth::validate_password(username, &password) {
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
    CommandResult {
        status: CommandStatus::Failure("Username not provided".into()),
        message: Some("530 Please enter the username first\r\n".into()),
        data: None,
    }
}

fn handle_cmd_list(client: &mut Client) -> CommandResult {
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    let client_addr = client.client_addr().unwrap();
    info!("Client {} requested directory listing", client_addr);

    match fs::read_dir("./test_dir") {
        Ok(entries) => {
            let mut file_list = vec![];

            for entry in entries.flatten() {
                file_list.push(entry.file_name().to_string_lossy().to_string());
            }

            CommandResult {
                status: CommandStatus::Success,
                message: Some("226 Transfer complete\r\n".into()),
                data: Some(CommandData::DirectoryListing(file_list)),
            }
        }
        Err(e) => {
            error!("Failed to read directory: {}", e);
            CommandResult {
                status: CommandStatus::Failure("550 Failed to list directory\r\n".into()),
                message: Some("550 Failed to list directory\r\n".into()),
                data: None,
            }
        }
    }
}

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

pub fn handle_cmd_stor(
    client: &mut Client,
    filename: &str,
    channel_registry: &mut ChannelRegistry,
) -> CommandResult {
    use log::{error, info};
    use std::fs;

    // 1. Validation
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }
    if !client.is_data_channel_init() {
        return CommandResult {
            status: CommandStatus::Failure("Data channel not initialized".into()),
            message: Some("530 Data channel not initialized\r\n".into()),
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
    if fs::metadata(filename).is_ok() {
        return CommandResult {
            status: CommandStatus::Failure("File exists".into()),
            message: Some("550 File already exists\r\n".into()),
            data: None,
        };
    }

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

    // 2. Setup data stream
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

    // 3. Pass TcpStream and filename to the actual file writing function
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

fn handle_cmd_retr(
    client: &mut Client,
    filename: &str,
    channel_registry: &mut ChannelRegistry,
) -> CommandResult {
    // 1. Validation
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }
    if !client.is_data_channel_init() {
        return CommandResult {
            status: CommandStatus::Failure("Data channel not initialized".into()),
            message: Some("530 Data channel not initialized\r\n".into()),
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
    if !fs::metadata(filename).is_ok() {
        return CommandResult {
            status: CommandStatus::Failure("File not found".into()),
            message: Some("550 File not found\r\n".into()),
            data: None,
        };
    }

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

    // 2. Setup data stream
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

    // 3. Pass TcpStream and filename to the actual file reading function
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
    match fs::remove_file(filename) {
        Ok(_) => CommandResult {
            status: CommandStatus::Success,
            message: Some("250 File deleted successfully\r\n".into()),
            data: None,
        },
        Err(e) => CommandResult {
            status: CommandStatus::Failure(e.to_string()),
            message: Some("550 Failed to delete file\r\n".into()),
            data: None,
        },
    }
}

fn handle_cmd_cwd(client: &Client, path: &String) -> CommandResult {
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }
    match env::set_current_dir(path) {
        Ok(_) => CommandResult {
            status: CommandStatus::Success,
            message: Some("250 Directory changed successfully\r\n".into()),
            data: None,
        },
        Err(e) => CommandResult {
            status: CommandStatus::Failure(e.to_string()),
            message: Some("550 Failed to change directory\r\n".into()),
            data: None,
        },
    }
}

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
        Err(e) => CommandResult {
            status: CommandStatus::Failure(e.to_string()),
            message: Some("550 Failed to get current directory\r\n".into()),
            data: None,
        },
    }
}

// Handle the PASV command to enter passive mode
// This function binds a socket for the data connection
// Does not return a data stream, just the socket address
fn handle_cmd_pasv(client: &mut Client, channel_registry: &mut ChannelRegistry) -> CommandResult {
    let client_addr = client.client_addr().unwrap().clone();
    // Step 1: Check if the client is logged in
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    // Step 2: Prevent duplicate initialization of the data channel
    if client.is_data_channel_init() {
        return CommandResult {
            status: CommandStatus::Failure("Data channel already initialized".into()),
            message: Some("425 Data connection already initialized\r\n".into()),
            data: None,
        };
    }

    // Step 3: Find the next available data socket address
    if let Some(data_socket) = channel_registry.next_available_socket() {
        // Step 4: Attempt to bind listener to the socket
        match TcpListener::bind(data_socket) {
            Ok(listener) => {
                // Step 5: Set listener to non-blocking
                if let Err(e) = listener.set_nonblocking(true) {
                    error!("Failed to set non-blocking mode: {}", e);
                    return CommandResult {
                        status: CommandStatus::Failure("Failed to configure listener".into()),
                        message: Some("425 Can't open data connection\r\n".into()),
                        data: None,
                    };
                }

                // Step 6: Update channel registry with new ChannelEntry and client
                let mut entry = ChannelEntry::default();

                entry.set_data_socket(Some(data_socket));
                entry.set_data_stream(None);
                entry.set_listener(Some(listener.try_clone().unwrap()));

                channel_registry.insert(client_addr, entry);
                client.set_data_channel_init(true);

                // Step 8: Log client and bound socket info clearly
                info!(
                    "Client {} bound to data socket {} in PASV mode",
                    client_addr, data_socket
                );

                // Step 9: Format PASV response in ip:port format
                let response = format!("227 Entering Passive Mode ({})\r\n", data_socket);

                // Step 10: Return success result
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

    // Step 11: No available port in range
    CommandResult {
        status: CommandStatus::Failure("No available port".into()),
        message: Some("425 Can't open data connection\r\n".into()),
        data: None,
    }
}

fn handle_cmd_port(
    client: &mut Client,
    channel_registry: &mut ChannelRegistry,
    addr: &String,
) -> CommandResult {
    let client_addr = client.client_addr().unwrap().clone();

    // Step 1: Check if the client is logged in
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    // Step 2: Parse the provided address
    match SocketAddr::from_str(addr) {
        Ok(data_socket) if data_socket.port() != 0 => {
            // Step 3: Check if the socket is already in use by another client
            if channel_registry.is_socket_taken(&data_socket) {
                error!(
                    "PORT command rejected: address {} already in use by another client",
                    data_socket
                );
                return CommandResult {
                    status: CommandStatus::Failure("Address in use".into()),
                    message: Some("425 Address already in use\r\n".into()),
                    data: None,
                };
            }

            // Step 4: Attempt to bind listener to the socket
            match TcpListener::bind(data_socket) {
                Ok(listener) => {
                    // Step 5: Set listener to non-blocking
                    if let Err(e) = listener.set_nonblocking(true) {
                        error!("Failed to set non-blocking mode: {}", e);
                        return CommandResult {
                            status: CommandStatus::Failure("Failed to configure listener".into()),
                            message: Some("425 Can't open data connection\r\n".into()),
                            data: None,
                        };
                    }

                    // Step 6: Update client state and registry
                    let mut entry = ChannelEntry::default();

                    entry.set_data_socket(Some(data_socket));
                    entry.set_data_stream(None);
                    entry.set_listener(Some(listener.try_clone().unwrap()));
                    channel_registry.insert(client_addr, entry);
                    client.set_data_channel_init(true);

                    // Step 7: Log success
                    info!(
                        "Client {} bound to data socket {} in PORT mode",
                        client_addr, data_socket
                    );

                    // Step 8: Return success
                    CommandResult {
                        status: CommandStatus::Success,
                        message: Some("200 PORT command successful\r\n".into()),
                        data: None,
                    }
                }
                Err(e) => {
                    error!("Failed to bind to {}: {}", data_socket, e);
                    CommandResult {
                        status: CommandStatus::Failure("Port binding failed".into()),
                        message: Some("425 Can't open data connection\r\n".into()),
                        data: None,
                    }
                }
            }
        }

        // Step 9: Invalid or malformed input
        _ => {
            error!("Invalid PORT address received from client {}", client_addr);
            CommandResult {
                status: CommandStatus::Failure("Invalid port".into()),
                message: Some("501 Invalid port\r\n".into()),
                data: None,
            }
        }
    }
}

fn handle_cmd_rax() -> CommandResult {
    CommandResult {
        status: CommandStatus::Success,
        message: Some("200 Rax is the best\r\n".into()),
        data: None,
    }
}

fn handle_cmd_unknown() -> CommandResult {
    CommandResult {
        status: CommandStatus::Failure("Unknown command".into()),
        message: Some("500 Syntax error, command unrecognized\r\n".into()),
        data: None,
    }
}
