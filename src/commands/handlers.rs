use crate::auth;
use crate::client::Client;
use crate::commands::parser::{Command, CommandData, CommandResult, CommandStatus};

use std::env;
use std::fs;
use std::net::SocketAddr;
use std::str::FromStr;

// Handle a single command and update server
pub fn handle_command(client: &mut Client, command: &Command) -> CommandResult {
    match command {
        Command::QUIT => handle_cmd_quit(client),
        Command::USER(username) => handle_cmd_user(client, username),
        Command::PASS(password) => handle_cmd_pass(client, password),
        Command::LIST => handle_cmd_list(client),
        Command::PWD => handle_cmd_pwd(client),
        Command::LOGOUT => handle_cmd_logout(client),
        Command::RETR(filename) => handle_cmd_retr(client, &filename),
        Command::STOR(filename) => handle_cmd_stor(client, &filename),
        Command::CWD(path) => handle_cmd_cwd(client, &path),
        Command::UNKNOWN(cmd) => handle_cmd_unknown(client, &cmd),
        Command::PASV() => handle_cmd_pasv(client),
        Command::PORT(addr) => handle_cmd_port(client, &addr),
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

fn handle_cmd_retr(client: &mut Client, filename: &String) -> CommandResult {
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
    if !fs::metadata(filename).is_ok() {
        return CommandResult {
            status: CommandStatus::Failure("File not found".into()),
            message: Some("550 File not found\r\n".into()),
            data: None,
        };
    }
    CommandResult {
        status: CommandStatus::Success,
        message: Some("150 Opening data connection\r\n".into()),
        data: Some(CommandData::File(filename.clone())),
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
    if !client.is_data_channel_init() {
        return CommandResult {
            status: CommandStatus::Failure("Data channel not initialized".into()),
            message: Some("530 Data channel not initialized\r\n".into()),
            data: None,
        };
    }
    CommandResult {
        status: CommandStatus::Success,
        message: Some("150 Opening data connection\r\n".into()),
        data: Some(CommandData::DirectoryListing(vec![])),
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

fn handle_cmd_unknown(client: &Client, cmd: &str) -> CommandResult {
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }
    let msg = if cmd == "rax" {
        "Rax is the best\r\n"
    } else {
        "500 Unknown command\r\n"
    };
    CommandResult {
        status: CommandStatus::Failure("Unknown command".into()),
        message: Some(msg.into()),
        data: None,
    }
}

fn handle_cmd_stor(client: &mut Client, filename: &String) -> CommandResult {
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
    CommandResult {
        status: CommandStatus::Success,
        message: Some("150 Opening data connection\r\n".into()),
        data: Some(CommandData::File(filename.clone())),
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

fn handle_cmd_pasv(client: &mut Client) -> CommandResult {
    const DATA_PORT_RANGE: std::ops::Range<u16> = 2122..2222;

    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }

    for port in DATA_PORT_RANGE {
        let socket_address = format!("127.0.0.1:{}", port).parse().unwrap();

        client.set_data_socket(Some(socket_address));
        client.set_data_port(Some(port));
        client.set_data_channel_init(true);

        let response = format!("227 Entering Passive Mode ({}:{})", "127.0.0.1", port);

        return CommandResult {
            status: CommandStatus::Success,
            message: Some(response),
            data: Some(CommandData::Connect(socket_address)),
        };
    }

    CommandResult {
        status: CommandStatus::Failure("No available port".into()),
        message: Some("425 Can't open data connection\r\n".into()),
        data: None,
    }
}

fn handle_cmd_port(client: &mut Client, addr: &String) -> CommandResult {
    if !client.is_logged_in() {
        return CommandResult {
            status: CommandStatus::Failure("Not logged in".into()),
            message: Some("530 Not logged in\r\n".into()),
            data: None,
        };
    }
    match SocketAddr::from_str(addr) {
        Ok(socket_address) if socket_address.port() != 0 => {
            client.set_data_socket(Some(socket_address));
            CommandResult {
                status: CommandStatus::Success,
                message: Some("200 PORT command successful\r\n".into()),
                data: Some(CommandData::Connect(socket_address)),
            }
        }
        _ => CommandResult {
            status: CommandStatus::Failure("Invalid port".into()),
            message: Some("501 Invalid port\r\n".into()),
            data: None,
        },
    }
}
