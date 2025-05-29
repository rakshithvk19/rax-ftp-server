use crate::server::ServerState;

use log::error;
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
    let raw = raw.trim();

    if raw == "QUIT" || raw == "q" {
        Command::Quit
    } else if raw.starts_with("USER ") {
        Command::User(raw.strip_prefix("USER ").unwrap_or("").trim().to_string())
    } else if raw.starts_with("PASS ") {
        Command::Pass(raw.strip_prefix("PASS ").unwrap_or("").trim().to_string())
    } else if raw == "LIST" {
        Command::List
    } else {
        Command::Unknown(raw.to_string())
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
            let _ = stream.write_all(b"221 Goodbye\r\n");
            return CommandResult::Quit;
        }
        Command::User(username) => {
            // state.auth.handle_user(&username, stream);
            auth_state.handle_user(&username, stream);
            return CommandResult::Wait;
        }
        Command::Pass(password) => {
            auth_state.handle_pass(&password, stream);
            return CommandResult::Wait;
        }
        Command::List => {
            if !auth_state.is_logged_in() {
                let _ = stream.write_all(b"530 Not logged in\r\n");
                return CommandResult::Wait;
            } else {
                let _ = stream.write_all(b"150 Opening data connection\r\n");

                match fs::read_dir(".") {
                    Ok(entries) => {
                        let mut file_list = String::new();

                        for entry in entries {
                            if let Ok(entry) = entry {
                                file_list.push_str(&format!(
                                    "{}\r\n",
                                    entry.file_name().to_string_lossy()
                                ));
                            }
                        }

                        let _ = stream.write_all(file_list.as_bytes());
                        let _ = stream.write_all(b"226 Transfer complete\r\n");
                    }
                    Err(e) => {
                        error!("Failed to read directory: {}", e);
                        let _ = stream.write_all(b"550 Failed to list directory\r\n");
                    }
                }
                return CommandResult::Continue;
            }
        }
        Command::Unknown(cmd) => {
            if !auth_state.is_logged_in() {
                let _ = stream.write_all(b"530 Not logged in\r\n");
            } else if cmd == "rax" {
                let _ = stream.write_all(b"Rax is the best\r\n");
            } else {
                let _ = stream.write_all(b"500 Unknown command\r\n");
            }
            return CommandResult::Continue;
        }
    }
}
