//! Module `file_transfer`
//!
//! Handles file upload and download operations for the FTP server.
//! Provides functionality to read from and write to files over
//! TCP data streams, managing errors and reporting FTP-compliant
//! status codes and messages.

use crate::command::CommandStatus;
use log::error;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;

/// Handles uploading a file from the client to the server.
///
/// This function reads data from the provided TCP data stream and writes it
/// into a file specified by `filename`. It reads data in chunks of 1024 bytes
/// until EOF is reached or an error occurs. Appropriate FTP command status
/// codes and messages are returned based on success or failure.
///
/// # Arguments
///
/// * `data_stream` - A TCP stream representing the data connection from client.
/// * `filename` - The path where the uploaded file will be saved on the server.
///
/// # Returns
///
/// * `Ok((CommandStatus, &str))` - On successful upload, returns FTP success status and message.
/// * `Err((CommandStatus, &str))` - On failure, returns FTP failure status and message.
///
/// # Errors
///
/// * Returns `550 Requested action not taken` if the file cannot be created.
/// * Returns `426 Connection closed; transfer aborted` if the data stream read fails.
/// * Returns `450 Requested file action not taken` if the file flush operation fails.
pub fn handle_file_upload(
    mut data_stream: TcpStream,
    filename: &str,
) -> Result<(CommandStatus, &'static str), (CommandStatus, &'static str)> {
    // Attempt to create the target file for writing the uploaded data.
    match File::create(filename) {
        Ok(mut file) => {
            let mut buffer = [0; 1024];

            // Continuously read from the data stream until EOF (0 bytes read).
            loop {
                match data_stream.read(&mut buffer) {
                    Ok(0) => break, // End of file / stream
                    Ok(n) => {
                        // Write the received data chunk into the file.
                        if let Err(e) = file.write_all(&buffer[..n]) {
                            error!("Failed to write to file {}: {}", filename, e);
                            return Err((
                                CommandStatus::Failure("550 Requested action not taken\r\n".into()),
                                "550 Requested action not taken\r\n",
                            ));
                        }
                    }
                    Err(e) => {
                        error!("Failed to read from data stream: {}", e);
                        return Err((
                            CommandStatus::Failure("426 Connection closed; transfer aborted\r\n".into()),
                            "426 Connection closed; transfer aborted\r\n",
                        ));
                    }
                }
            }

            // Flush buffered data to disk before closing the file.
            if file.flush().is_ok() {
                Ok((CommandStatus::Success, "226 Transfer complete\r\n"))
            } else {
                Err((
                    CommandStatus::Failure("450 Requested file action not taken\r\n".into()),
                    "450 Requested file action not taken\r\n",
                ))
            }
        }
        Err(e) => {
            error!("Failed to create file {}: {}", filename, e);
            Err((
                CommandStatus::Failure("550 Requested action not taken\r\n".into()),
                "550 Requested action not taken\r\n",
            ))
        }
    }
}

/// Handles downloading a file from the server to the client.
///
/// This function reads the contents of the specified file and writes
/// it to the provided TCP data stream in chunks of 1024 bytes until
/// the entire file is sent or an error occurs. Appropriate FTP
/// command status codes and messages are returned based on success or failure.
///
/// # Arguments
///
/// * `data_stream` - A TCP stream representing the data connection to client.
/// * `filename` - The path of the file to be downloaded from the server.
///
/// # Returns
///
/// * `Ok((CommandStatus, &str))` - On successful download, returns FTP success status and message.
/// * `Err((CommandStatus, &str))` - On failure, returns FTP failure status and message.
///
/// # Errors
///
/// * Returns `550 Failed to open file` if the file cannot be opened.
/// * Returns `451 Requested action aborted` if file reading fails.
/// * Returns `426 Connection closed; transfer aborted` if writing to the data stream fails.
/// * Returns `450 Requested file action not taken` if flushing the data stream fails.
pub fn handle_file_download(
    mut data_stream: TcpStream,
    filename: &str,
) -> Result<(CommandStatus, &'static str), (CommandStatus, &'static str)> {
    // Attempt to open the file for reading
    match File::open(filename) {
        Ok(mut file) => {
            let mut buffer = [0; 1024];

            // Continuously read from the file and write to the data stream until EOF
            loop {
                match file.read(&mut buffer) {
                    Ok(0) => break, // EOF reached
                    Ok(n) => {
                        // Write the chunk read from the file into the data stream
                        if let Err(e) = data_stream.write_all(&buffer[..n]) {
                            error!("Failed to write to data stream: {}", e);
                            return Err((
                                CommandStatus::Failure("426 Connection closed; transfer aborted\r\n".into()),
                                "426 Connection closed; transfer aborted\r\n",
                            ));
                        }
                    }
                    Err(e) => {
                        error!("Failed to read from file {}: {}", filename, e);
                        return Err((
                            CommandStatus::Failure("451 Requested action aborted\r\n".into()),
                            "451 Requested action aborted\r\n",
                        ));
                    }
                }
            }

            // Flush the data stream to ensure all data is sent
            if data_stream.flush().is_ok() {
                Ok((CommandStatus::Success, "226 Transfer complete\r\n"))
            } else {
                Err((
                    CommandStatus::Failure("450 Requested file action not taken\r\n".into()),
                    "450 Requested file action not taken\r\n",
                ))
            }
        }
        Err(e) => {
            error!("Failed to open file {}: {}", filename, e);
            Err((
                CommandStatus::Failure("550 Failed to open file\r\n".into()),
                "550 Failed to open file\r\n",
            ))
        }
    }
}
