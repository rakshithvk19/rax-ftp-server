// src/server.rs
use log::{error, info};
use std::io::{Read, Write};
use std::net::TcpStream;

pub fn handle_client(mut stream: TcpStream) {
    // Send welcome message
    if let Err(e) = stream.write_all(b"220 Welcome to the FTP server\r\n") {
        error!("Failed to send welcome: {}", e);
        return;
    }

    let mut buffer = [0; 1024];
    let mut command_buffer = String::new();
    let mut is_user_valid = false;
    let mut is_logged_in = false;

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                info!("Connection closed by client");
                break;
            }
            Ok(n) => {
                // Append received data to command_buffer
                command_buffer.push_str(&String::from_utf8_lossy(&buffer[..n]));

                // Check for complete command (ends with \r\n)
                if command_buffer.ends_with("\r\n") {
                    let command = command_buffer
                        .trim_end_matches(&['\r', '\n'][..])
                        .to_string();

                    info!(
                        "Received from {}: {:?}",
                        stream.peer_addr().unwrap(),
                        command
                    );

                    if command == "QUIT" || command == "q" {
                        let _ = stream.write_all(b"221 Goodbye\r\n");
                        break;
                    } else if command.starts_with("USER ") {
                        let username = command.strip_prefix("USER ").unwrap_or("").trim();
                        if username == "user" {
                            is_user_valid = true;
                            let _ = stream.write_all(b"331 Password required\r\n");
                        } else {
                            is_user_valid = false;
                            is_logged_in = false;
                            let _ = stream.write_all(b"530 Invalid username\r\n");
                        }
                    } else if command.starts_with("PASS ") {
                        if is_user_valid {
                            let password = command.strip_prefix("PASS ").unwrap_or("").trim();
                            if password == "pass" {
                                is_logged_in = true;
                                let _ = stream.write_all(b"230 Login successful\r\n");
                            } else {
                                is_logged_in = false;
                                let _ = stream.write_all(b"530 Invalid password\r\n");
                            }
                        } else {
                            let _ = stream.write_all(b"530 Please send USER first\r\n");
                        }
                    } else if !is_logged_in {
                        let _ = stream.write_all(b"530 Not logged in\r\n");
                    } else if command == "rax" {
                        let _ = stream.write_all(b"Rax is the best\r\n");
                    } else {
                        let _ = stream.write_all(b"500 Unknown command\r\n");
                    }
                    command_buffer.clear(); // Reset for next command
                }
            }

            Err(e) => {
                error!("Failed to read from stream: {}", e);
                break;
            }
        }
    }
}
