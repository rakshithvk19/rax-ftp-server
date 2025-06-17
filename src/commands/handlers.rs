use crate::client::Client;
use crate::commands::parser::{Command, CommandResult};

use log::{error, info};
use std::env;
use std::fs;
use std::io::Write;
use std::net::TcpStream;

//TODO: use require_auth from utils.rs

// Handle a single command and update server
pub fn handle_command(
    client: &mut Client,
    command: Command,
    stream: &mut TcpStream,
) -> CommandResult {
    // let client = server.get_client();

    match command {
        Command::Quit => handle_cmd_quit(client, stream),
        Command::User(username) => handle_cmd_user(client, username, stream),
        Command::Pass(password) => handle_cmd_pass(client, password, stream),
        Command::List => handle_cmd_list(client, stream),
        Command::Pwd => handle_cmd_pwd(client, stream),
        Command::Logout => handle_cmd_logout(client, stream),
        Command::Retr(filename) => handle_cmd_retr(client, &filename, stream),
        Command::Stor(filename) => handle_cmd_stor(client, &filename, stream),
        Command::Cwd(path) => handle_cmd_cwd(client, &path, stream),
        Command::Unknown(cmd) => handle_cmd_unknown(client, stream, &cmd),
        Command::PASV() => handle_cmd_pasv(client, stream),
    }
}

// Command handler for QUIT
fn handle_cmd_quit(_client: &mut Client, stream: &mut TcpStream) -> CommandResult {
    let _ = stream.write_all(b"221 Goodbye\r\n");
    CommandResult::Quit
}

// Command handler for PASS
fn handle_cmd_pass(client: &mut Client, password: String, stream: &mut TcpStream) -> CommandResult {
    client.handle_pass(&password, stream);
    CommandResult::Continue
}

// Command handler for RETR
fn handle_cmd_retr(
    client: &mut Client,
    filename: &String,
    stream: &mut TcpStream,
) -> CommandResult {
    if !client.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        CommandResult::Continue
    } else if !client.is_data_channel_init() {
        let _ = stream.write_all(b"530 Data channel not initialized\r\n");
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
fn handle_cmd_user(client: &mut Client, username: String, stream: &mut TcpStream) -> CommandResult {
    client.handle_user(&username, stream);
    CommandResult::Continue
}

// Command handler for LIST
fn handle_cmd_list(client: &mut Client, stream: &mut TcpStream) -> CommandResult {
    if !client.is_logged_in() {
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

fn handle_cmd_logout(client: &mut Client, stream: &mut TcpStream) -> CommandResult {
    if client.is_logged_in() {
        client.logout();
        let _ = stream.write_all(b"221 Logout successful\r\n");
    } else {
        let _ = stream.write_all(b"530 User Not logged in\r\n");
    }
    CommandResult::Continue
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
    CommandResult::Continue
}

fn handle_cmd_stor(
    client: &mut Client,
    filename: &String,
    stream: &mut TcpStream,
) -> CommandResult {
    // user client
    if !client.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        CommandResult::Continue
    } else if !client.is_data_channel_init() {
        let _ = stream.write_all(b"530 Data channel not initialized\r\n");
        CommandResult::Continue
    } else {
        //TODO: Write better filename validation
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

// Command handler for CWD (Change Working Directory)
//TODO: Improve error handling and path validation
fn handle_cmd_cwd(client: &Client, path: &String, stream: &mut TcpStream) -> CommandResult {
    if !client.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        CommandResult::Continue
    } else {
        match env::set_current_dir(path) {
            Ok(_) => {
                let _ = stream.write_all(b"250 Directory changed successfully\r\n");
                CommandResult::Continue
            }
            Err(e) => {
                error!("Failed to change directory: {}", e);
                let _ = stream.write_all(b"550 Failed to change directory\r\n");
                CommandResult::Continue
            }
        }
    }
}

fn handle_cmd_pwd(client: &Client, stream: &mut TcpStream) -> CommandResult {
    if !client.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        CommandResult::Continue
    } else {
        match env::current_dir() {
            Ok(path) => {
                let response = format!("257 \"{}\"\r\n", path.display());
                let _ = stream.write_all(response.as_bytes());
                CommandResult::Continue
            }
            Err(e) => {
                error!("Failed to get current directory: {}", e);
                let _ = stream.write_all(b"550 Failed to get current directory\r\n");
                CommandResult::Continue
            }
        }
    }
}

fn handle_cmd_pasv(client: &Client, stream: &mut TcpStream) -> CommandResult {
    if !client.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        CommandResult::Continue
    } else {
        CommandResult::CONNECT
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::server::Server;
//     use std::net::TcpListener;

//     fn setup() -> (Server, TcpStream, TcpStream) {
//         let listener = TcpListener::bind("127.0.0.1:0").unwrap();
//         let addr = listener.local_addr().unwrap();

//         let client = TcpStream::connect(addr).unwrap();
//         let (cmd_stream, _) = listener.accept().unwrap();

//         let mut server = Server::new();
//         (server, )
//         (cmd_stream, client, server)
//     }

//     #[test]
//     fn test_handle_quit() {
//         let (mut server, _client, mut server) = setup();
//         let result = handle_command(&mut server, Command::Quit, &mut server);
//         assert_eq!(result, CommandResult::Quit);
//     }

//     #[test]
//     fn test_handle_user() {
//         let (mut server, _client, mut server) = setup();
//         let result = handle_command(&mut server, Command::User("test".to_string()), &mut server);
//         assert_eq!(result, CommandResult::Continue);
//         assert!(!server.get_client().is_logged_in());
//     }

//     #[test]
//     fn test_handle_pass() {
//         let (mut server, _client, mut server) = setup();

//         // First set username
//         handle_command(&mut server, Command::User("test".to_string()), &mut server);

//         // Then try password
//         let result = handle_command(&mut server, Command::Pass("pass".to_string()), &mut server);
//         assert_eq!(result, CommandResult::Continue);
//     }

//     #[test]
//     fn test_handle_list_unauthorized() {
//         let (mut server, _client, mut server) = setup();
//         let result = handle_command(&mut server, Command::List, &mut server);
//         assert_eq!(result, CommandResult::Continue);
//     }

//     #[test]
//     fn test_handle_pwd_unauthorized() {
//         let (mut server, _client, mut server) = setup();
//         let result = handle_command(&mut server, Command::Pwd, &mut server);
//         assert_eq!(result, CommandResult::Continue);
//     }

//     #[test]
//     fn test_handle_retr_unauthorized() {
//         let (mut server, _client, mut server) = setup();
//         let result = handle_command(
//             &mut server,
//             Command::Retr("test.txt".to_string()),
//             &mut server,
//         );
//         assert_eq!(result, CommandResult::Continue);
//     }

//     #[test]
//     fn test_handle_stor_unauthorized() {
//         let (mut server, _client, mut server) = setup();
//         let result = handle_command(
//             &mut server,
//             Command::Stor("test.txt".to_string()),
//             &mut server,
//         );
//         assert_eq!(result, CommandResult::Continue);
//     }

//     #[test]
//     fn test_handle_cwd_unauthorized() {
//         let (mut server, _client, mut server) = setup();
//         let result = handle_command(&mut server, Command::Cwd(".".to_string()), &mut server);
//         assert_eq!(result, CommandResult::Continue);
//     }

//     #[test]
//     fn test_handle_stor_invalid_filename() {
//         let (mut server, _client, mut server) = setup();
//         // server.get_client().login();

//         let result = handle_command(
//             &mut server,
//             Command::Stor("../test.txt".to_string()),
//             &mut server,
//         );
//         assert_eq!(result, CommandResult::Continue);
//     }

//     #[test]
//     fn test_handle_unknown() {
//         let (mut server, _client, mut server) = setup();
//         let result = handle_command(
//             &mut server,
//             Command::Unknown("invalid".to_string()),
//             &mut server,
//         );
//         assert_eq!(result, CommandResult::Continue);
//     }

//     #[test]
//     fn test_handle_logout() {
//         let (mut server, _client, mut server) = setup();
//         // server.get_client().login();

//         let result = handle_command(&mut server, Command::Logout, &mut server);
//         assert_eq!(result, CommandResult::Continue);
//         assert!(!server.get_client().is_logged_in());
//     }
// }
