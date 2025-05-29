use crate::auth::AuthState;
use crate::server::ServerState;

use log::{error, info};
use std::fs;
use std::io::Write;
use std::net::TcpStream;

// Command enum to represent FTP commands
#[derive(Debug, PartialEq)]
pub enum Command {
    Quit,
    User(String),
    Pass(String),
    List,
    Retr(String),
    Unknown(String),
}

#[derive(Debug, PartialEq)]
pub enum CommandResult {
    Quit,
    Wait,
    Continue,
}

// Parse raw command string into Command enum
pub fn parse_command(raw: &str) -> Command {
    let trimmed = raw.trim();
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let cmd = parts.next().unwrap_or("").to_ascii_uppercase();
    let arg = parts.next().unwrap_or("").trim();

    match cmd.as_str() {
        "QUIT" | "Q" => Command::Quit,
        "USER" => Command::User(arg.to_string()),
        "PASS" => Command::Pass(arg.to_string()),
        "LIST" => Command::List,
        "RETR" => Command::Retr(arg.to_string()),
        _ => Command::Unknown(trimmed.to_string()),
    }
}

// Handle a single command and update state
pub fn handle_command(
    state: &mut ServerState,
    command: Command,
    stream: &mut TcpStream,
) -> CommandResult {
    let auth_state = state.get_auth();

    match command {
        Command::Quit => {
            return handle_quit(auth_state, stream);
        }
        Command::User(username) => {
            return handle_cmd_user(auth_state, username, stream);
        }
        Command::Pass(password) => {
            return handle_pass(auth_state, password, stream);
        }
        Command::List => {
            return handle_list(auth_state, stream);
        }
        Command::Retr(filename) => {
            return handle_retr(auth_state, &filename, stream);
        }
        Command::Unknown(cmd) => {
            return handle_unknown(auth_state, stream, &cmd);
        }
    }
}

// Command handler for QUIT
fn handle_quit(_auth_state: &mut AuthState, stream: &mut TcpStream) -> CommandResult {
    let _ = stream.write_all(b"221 Goodbye\r\n");
    return CommandResult::Quit;
}

// Command handler for PASS
fn handle_pass(
    auth_state: &mut AuthState,
    password: String,
    stream: &mut TcpStream,
) -> CommandResult {
    auth_state.handle_pass(&password, stream);
    return CommandResult::Wait;
}

// Command handler for RETR
fn handle_retr(
    auth_state: &mut AuthState,
    filename: &String,
    stream: &mut TcpStream,
) -> CommandResult {
    if !auth_state.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        return CommandResult::Continue;
    } else {
        //Check if input is file or dir

        //Check if file exists in current directory
        if !fs::metadata(&filename).is_ok() {
            let _ = stream.write_all(b"550 File not found \r\n");
            return CommandResult::Continue;
        } else {
            let _ = stream.write_all(b"150 Opening data connection\r\n");

            // Open the file and stream its content
            match fs::File::open(&filename) {
                Ok(mut file) => {
                    if let Err(e) = std::io::copy(&mut file, stream) {
                        error!("Failed to read file: {}", e);
                        let _ = stream.write_all(
                            b"451 Requested action aborted: local error in processing.\r\n",
                        );
                    } else {
                        match stream.flush() {
                            Ok(_) => {
                                info!("File {} sent successfully", filename);
                                let _ = stream.write_all(b"226 Transfer complete\r\n");
                            }
                            Err(e) => {
                                error!("Failed to flush stream: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to open file: {}", e);
                    let _ = stream.write_all(b"550 Failed to open file\r\n");
                }
            }
        }
    }
    return CommandResult::Continue;
}

// Command handler for USER
fn handle_cmd_user(
    auth_state: &mut AuthState,
    username: String,
    stream: &mut TcpStream,
) -> CommandResult {
    auth_state.handle_user(&username, stream);
    return CommandResult::Wait;
}

// Command handler for LIST
fn handle_list(auth_state: &mut AuthState, stream: &mut TcpStream) -> CommandResult {
    if !auth_state.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        return CommandResult::Continue;
    } else {
        let _ = stream.write_all(b"150 Opening data connection\r\n");

        match fs::read_dir(".") {
            Ok(entries) => {
                let mut file_list = String::new();

                for entry in entries {
                    if let Ok(entry) = entry {
                        file_list.push_str(&format!("{}\r\n", entry.file_name().to_string_lossy()));
                    }
                }

                let _ = stream.write_all(file_list.as_bytes());
            }
            Err(e) => {
                error!("Failed to read directory: {}", e);
                let _ = stream.write_all(b"550 Failed to list directory\r\n");
            }
        }
        return CommandResult::Continue;
    }
}

// Command handler for unknown commands
fn handle_unknown(auth_state: &mut AuthState, stream: &mut TcpStream, cmd: &str) -> CommandResult {
    if !auth_state.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
    } else if cmd == "rax" {
        let _ = stream.write_all(b"Rax is the best\r\n");
    } else {
        let _ = stream.write_all(b"500 Unknown command\r\n");
    }
    return CommandResult::Continue;
}
