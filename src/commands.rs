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
    List,
    Logout,
    User(String),
    Pass(String),
    Retr(String),
    Stor(String),
    Unknown(String),
}

#[derive(Debug, PartialEq)]
pub enum CommandResult {
    Quit,
    Continue,
    Stor,
}

// Parse raw command string into Command enum
pub fn parse_command(raw: &str) -> Command {
    let trimmed = raw.trim();
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let cmd = parts.next().unwrap_or("").to_ascii_uppercase();
    let arg = parts.next().unwrap_or("").trim();

    match cmd.as_str() {
        "QUIT" | "Q" => Command::Quit,
        "LIST" => Command::List,
        "LOGOUT" => Command::Logout,
        "USER" => Command::User(arg.to_string()),
        "PASS" => Command::Pass(arg.to_string()),
        "RETR" => Command::Retr(arg.to_string()),
        "STOR" => Command::Stor(arg.to_string()),
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
        Command::Quit => handle_cmd_quit(auth_state, stream),
        Command::User(username) => handle_cmd_user(auth_state, username, stream),
        Command::Pass(password) => handle_cmd_pass(auth_state, password, stream),
        Command::List => handle_cmd_list(auth_state, stream),
        Command::Logout => handle_cmd_logout(auth_state, stream),
        Command::Retr(filename) => handle_cmd_retr(auth_state, &filename, stream),
        Command::Stor(filename) => handle_cmd_stor(auth_state, &filename, stream),
        Command::Unknown(cmd) => handle_cmd_unknown(auth_state, stream, &cmd),
    }
}

// Command handler for QUIT
fn handle_cmd_quit(_auth_state: &mut AuthState, stream: &mut TcpStream) -> CommandResult {
    let _ = stream.write_all(b"221 Goodbye\r\n");
    CommandResult::Quit
}

// Command handler for PASS
fn handle_cmd_pass(
    auth_state: &mut AuthState,
    password: String,
    stream: &mut TcpStream,
) -> CommandResult {
    auth_state.handle_pass(&password, stream);
    CommandResult::Continue
}

// Command handler for RETR
fn handle_cmd_retr(
    auth_state: &mut AuthState,
    filename: &String,
    stream: &mut TcpStream,
) -> CommandResult {
    if !auth_state.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        CommandResult::Continue
    } else {
        if !fs::metadata(&filename).is_ok() {
            let _ = stream.write_all(b"550 File not found \r\n");
            CommandResult::Continue
        } else {
            let _ = stream.write_all(b"150 Opening data connection\r\n");

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
            CommandResult::Continue
        }
    }
}

// Command handler for USER
fn handle_cmd_user(
    auth_state: &mut AuthState,
    username: String,
    stream: &mut TcpStream,
) -> CommandResult {
    auth_state.handle_user(&username, stream);
    CommandResult::Continue
}

// Command handler for LIST
fn handle_cmd_list(auth_state: &mut AuthState, stream: &mut TcpStream) -> CommandResult {
    if !auth_state.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        CommandResult::Continue
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
        CommandResult::Continue
    }
}

fn handle_cmd_logout(auth_state: &mut AuthState, stream: &mut TcpStream) -> CommandResult {
    if auth_state.is_logged_in() {
        auth_state.logout();
        let _ = stream.write_all(b"221 Logout successful\r\n");
    } else {
        let _ = stream.write_all(b"530 User Not logged in\r\n");
    }
    CommandResult::Continue
}

// Command handler for unknown commands
fn handle_cmd_unknown(
    auth_state: &mut AuthState,
    stream: &mut TcpStream,
    cmd: &str,
) -> CommandResult {
    if !auth_state.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
    } else if cmd == "rax" {
        let _ = stream.write_all(b"Rax is the best\r\n");
    } else {
        let _ = stream.write_all(b"500 Unknown command\r\n");
    }
    CommandResult::Continue
}

fn handle_cmd_stor(
    auth_state: &mut AuthState,
    filename: &String,
    stream: &mut TcpStream,
) -> CommandResult {
    // user auth
    if !auth_state.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        CommandResult::Continue
    } else {
        // Filename validation
        if filename.is_empty() {
            let _ = stream.write_all(b"501 Syntax error in parameters or arguments\r\n");
            CommandResult::Continue
        } else if filename.contains("..")
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
            CommandResult::Continue
        } else if fs::metadata(&filename).is_ok() {
            let _ = stream.write_all(b"550 File already exists\r\n");
            CommandResult::Stor
        } else {
            match fs::File::create(filename) {
                Ok(_) => {
                    info!("File {} created successfully", filename);
                    let _ = stream.write_all(b"226 Transfer complete\r\n");
                    let _ = stream.flush();
                    CommandResult::Stor
                }
                Err(e) => {
                    error!("Failed to create file: {}", e);
                    let _ = stream.write_all(b"550 Failed to create file\r\n");
                    let _ = stream.flush();
                    CommandResult::Continue
                }
            }
        }
    }
}
