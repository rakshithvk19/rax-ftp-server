//! Module `file_transfer`
//!
//! Handles file upload and download operations for the FTP server.
//! Provides functionality to read from and write to files over
//! TCP data streams, managing errors and reporting FTP-compliant
//! status codes and messages.

use crate::command::CommandStatus;
use log::{error, warn};
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

const MAX_RETRIES: usize = 3;

/// Sanitizes filename input to prevent directory traversal attacks.
fn sanitize_filename(filename: &str) -> Option<String> {
    if filename.contains("..")
        || filename.contains('\0')
        || filename.contains('/')
        || filename.contains('\\')
    {
        warn!("Rejected suspicious filename: {}", filename);
        None
    } else {
        Some(filename.trim().to_string())
    }
}

/// Handles uploading a file from the client to the server.
pub fn handle_file_upload(
    mut data_stream: TcpStream,
    filename: &str,
) -> Result<(CommandStatus, &'static str), (CommandStatus, &'static str)> {
    let sanitized = match sanitize_filename(filename) {
        Some(name) => name,
        None => {
            return Err((
                CommandStatus::Failure(
                    "550 Invalid filename
"
                    .into(),
                ),
                "550 Invalid filename
",
            ));
        }
    };

    match File::create(&sanitized) {
        Ok(mut file) => {
            let mut buffer = [0; 1024];
            loop {
                let mut retries = 0;
                let n = loop {
                    match data_stream.read(&mut buffer) {
                        Ok(0) => break 0, // EOF
                        Ok(n) => break n,
                        Err(e) if retries < MAX_RETRIES => {
                            warn!("Transient read error: {}. Retrying...", e);
                            retries += 1;
                            thread::sleep(Duration::from_millis(100));
                        }
                        Err(e) => {
                            error!("Read failure: {}", e);
                            return Err((
                                CommandStatus::Failure(
                                    "426 Connection closed; transfer aborted
"
                                    .into(),
                                ),
                                "426 Connection closed; transfer aborted
",
                            ));
                        }
                    }
                };
                if n == 0 {
                    break;
                }

                if let Err(e) = file.write_all(&buffer[..n]) {
                    error!("Failed to write to {}: {}", sanitized, e);
                    return Err((
                        CommandStatus::Failure(
                            "550 Requested action not taken
"
                            .into(),
                        ),
                        "550 Requested action not taken
",
                    ));
                }
            }

            if file.flush().is_ok() {
                Ok((
                    CommandStatus::Success,
                    "226 Transfer complete
",
                ))
            } else {
                Err((
                    CommandStatus::Failure(
                        "450 Requested file action not taken
"
                        .into(),
                    ),
                    "450 Requested file action not taken
",
                ))
            }
        }
        Err(e) => {
            error!("File create failed for {}: {}", sanitized, e);
            Err((
                CommandStatus::Failure("550 Requested action not taken".into()),
                "550 Requested action not taken",
            ))
        }
    }
}

/// Handles downloading a file from the server to the client.
pub fn handle_file_download(
    mut data_stream: TcpStream,
    filename: &str,
) -> Result<(CommandStatus, &'static str), (CommandStatus, &'static str)> {
    let sanitized = match sanitize_filename(filename) {
        Some(name) => name,
        None => {
            return Err((
                CommandStatus::Failure("550 Invalid filename".into()),
                "550 Invalid filename",
            ));
        }
    };

    match File::open(&sanitized) {
        Ok(mut file) => {
            let mut buffer = [0; 1024];
            loop {
                let n = match file.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) => {
                        error!("Read error on {}: {}", sanitized, e);
                        return Err((
                            CommandStatus::Failure("451 Requested action aborted".into()),
                            "451 Requested action aborted",
                        ));
                    }
                };

                let mut retries = 0;
                loop {
                    match data_stream.write_all(&buffer[..n]) {
                        Ok(_) => break,
                        Err(e) if retries < MAX_RETRIES => {
                            warn!("Transient write error: {}. Retrying...", e);
                            retries += 1;
                            thread::sleep(Duration::from_millis(100));
                        }
                        Err(e) => {
                            error!("Write failure to data stream: {}", e);
                            return Err((
                                CommandStatus::Failure(
                                    "426 Connection closed; transfer aborted".into(),
                                ),
                                "426 Connection closed; transfer aborted",
                            ));
                        }
                    }
                }
            }

            if data_stream.flush().is_ok() {
                Ok((CommandStatus::Success, "226 Transfer complete"))
            } else {
                Err((
                    CommandStatus::Failure("450 Requested file action not taken".into()),
                    "450 Requested file action not taken",
                ))
            }
        }
        Err(e) => {
            error!("Failed to open file {}: {}", sanitized, e);
            Err((
                CommandStatus::Failure("550 Failed to open file".into()),
                "550 Failed to open file",
            ))
        }
    }
}
