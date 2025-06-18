use crate::client::Client;
use crate::commands::parser::{Command, CommandResult};

use log::error;
use std::env;
use std::fs;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::str::FromStr;

//TODO: use require_auth from utils.rs

// Handle a single command and update server
pub fn handle_command(
    client: &mut Client,
    command: Command,
    stream: &mut TcpStream,
) -> CommandResult {
    // let client = server.get_client();

    match command {
        Command::QUIT => handle_cmd_quit(client, stream),
        Command::USER(username) => handle_cmd_user(client, username, stream),
        Command::PASS(password) => handle_cmd_pass(client, password, stream),
        Command::LIST => handle_cmd_list(client, stream),
        Command::PWD => handle_cmd_pwd(client, stream),
        Command::LOGOUT => handle_cmd_logout(client, stream),
        Command::RETR(filename) => handle_cmd_retr(client, &filename, stream),
        Command::STOR(filename) => handle_cmd_stor(client, &filename, stream),
        Command::CWD(path) => handle_cmd_cwd(client, &path, stream),
        Command::UNKNOWN(cmd) => handle_cmd_unknown(client, stream, &cmd),
        Command::PASV() => handle_cmd_pasv(client, stream),
        Command::PORT(addr) => handle_cmd_port(client, &addr, stream),
    }
}

// Command handler for QUIT
fn handle_cmd_quit(_client: &mut Client, stream: &mut TcpStream) -> CommandResult {
    let _ = stream.write_all(b"221 Goodbye\r\n");
    CommandResult::QUIT
}

// Command handler for PASS
fn handle_cmd_pass(client: &mut Client, password: String, stream: &mut TcpStream) -> CommandResult {
    client.handle_pass(&password, stream);
    CommandResult::CONTINUE
}

fn handle_cmd_retr(
    client: &mut Client,
    filename: &String,
    stream: &mut TcpStream,
) -> CommandResult {
    if !client.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        return CommandResult::CONTINUE;
    }
    if !client.is_data_channel_init() {
        let _ = stream.write_all(b"530 Data channel not initialized\r\n");
        return CommandResult::CONTINUE;
    }
    if !fs::metadata(filename).is_ok() {
        let _ = stream.write_all(b"550 File not found\r\n");
        return CommandResult::CONTINUE;
    }
    let _ = stream.write_all(b"150 Opening data connection\r\n");
    CommandResult::RETR(filename.clone())
}

// Command handler for USER
fn handle_cmd_user(client: &mut Client, username: String, stream: &mut TcpStream) -> CommandResult {
    client.handle_user(&username, stream);
    CommandResult::CONTINUE
}

fn handle_cmd_list(client: &mut Client, stream: &mut TcpStream) -> CommandResult {
    if !client.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        return CommandResult::CONTINUE;
    }
    if !client.is_data_channel_init() {
        let _ = stream.write_all(b"530 Data channel not initialized\r\n");
        return CommandResult::CONTINUE;
    }
    let _ = stream.write_all(b"150 Opening data connection\r\n");
    CommandResult::LIST
}

fn handle_cmd_logout(client: &mut Client, stream: &mut TcpStream) -> CommandResult {
    if client.is_logged_in() {
        client.logout();
        let _ = stream.write_all(b"221 Logout successful\r\n");
    } else {
        let _ = stream.write_all(b"530 User Not logged in\r\n");
    }
    CommandResult::CONTINUE
}

// Command handler for unknown commands
fn handle_cmd_unknown(client: &mut Client, stream: &mut TcpStream, cmd: &str) -> CommandResult {
    if !client.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
    } else if cmd == "rax" {
        let _ = stream.write_all(b"Rax is the best\r\n");
    } else {
        let _ = stream.write_all(b"500 Unknown command\r\n");
    }
    CommandResult::CONTINUE
}

fn handle_cmd_stor(
    client: &mut Client,
    filename: &String,
    stream: &mut TcpStream,
) -> CommandResult {
    if !client.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        return CommandResult::CONTINUE;
    }
    if !client.is_data_channel_init() {
        let _ = stream.write_all(b"530 Data channel not initialized\r\n");
        return CommandResult::CONTINUE;
    }
    if filename.is_empty() {
        let _ = stream.write_all(b"501 Syntax error in parameters or arguments\r\n");
        return CommandResult::CONTINUE;
    }
    if filename.contains("..")
        || filename.contains("/")
        || filename.contains("\\")
        || filename.contains(":")
        || filename.contains("*")
        || filename.contains("?")
        || filename.contains("\"")
        || filename.contains("<")
        || filename.contains(">")
        || filename.contains("|")
    {
        let _ = stream.write_all(b"550 Filename invalid\r\n");
        return CommandResult::CONTINUE;
    }
    if fs::metadata(filename).is_ok() {
        let _ = stream.write_all(b"550 File already exists\r\n");
        return CommandResult::CONTINUE;
    }
    let _ = stream.write_all(b"150 Opening data connection\r\n");
    CommandResult::STOR(filename.clone())
}
// Command handler for CWD (Change Working Directory)
//TODO: Improve error handling and path validation
fn handle_cmd_cwd(client: &Client, path: &String, stream: &mut TcpStream) -> CommandResult {
    if !client.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        CommandResult::CONTINUE
    } else {
        match env::set_current_dir(path) {
            Ok(_) => {
                let _ = stream.write_all(b"250 Directory changed successfully\r\n");
                CommandResult::CONTINUE
            }
            Err(e) => {
                error!("Failed to change directory: {}", e);
                let _ = stream.write_all(b"550 Failed to change directory\r\n");
                CommandResult::CONTINUE
            }
        }
    }
}

fn handle_cmd_pwd(client: &Client, stream: &mut TcpStream) -> CommandResult {
    if !client.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        CommandResult::CONTINUE
    } else {
        match env::current_dir() {
            Ok(path) => {
                let response = format!("257 \"{}\"\r\n", path.display());
                let _ = stream.write_all(response.as_bytes());
                CommandResult::CONTINUE
            }
            Err(e) => {
                error!("Failed to get current directory: {}", e);
                let _ = stream.write_all(b"550 Failed to get current directory\r\n");
                CommandResult::CONTINUE
            }
        }
    }
}

fn handle_cmd_pasv(client: &Client, stream: &mut TcpStream) -> CommandResult {
    if !client.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        CommandResult::CONTINUE
    } else {
        CommandResult::CONNECT(None)
    }
}

fn handle_cmd_port(client: &mut Client, addr: &String, stream: &mut TcpStream) -> CommandResult {
    if !client.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        return CommandResult::CONTINUE;
    }

    // if !client.is_data_channel_init() {
    //     let _ = stream.write_all(b"530 Data channel not initialized\r\n");
    //     return CommandResult::CONTINUE;
    // }

    //Parse port into socket
    match SocketAddr::from_str(addr) {
        Ok(socket_address) => {
            if socket_address.port() == 0 {
                let _ = stream.write_all(b"501 Invalid port\r\n");
                return CommandResult::CONTINUE;
            } else {
                client.set_data_socket(Some(socket_address));
                return CommandResult::CONNECT(Some(socket_address));
            }
        }
        //TODO: Handle addr parse error
        Err(e) => {
            let _ = stream.write_all(
                b"501 Invalid port. Enter the port in the following format -> ip::port. Current error \r\n",
            );
            return CommandResult::CONTINUE;
        }
    }
}
