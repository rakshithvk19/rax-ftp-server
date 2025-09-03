//! Module `file_transfer`
//!
//! Handles file upload and download operations for the FTP server.
//! Provides functionality to read from and write to files over
//! TCP data streams, managing errors and reporting FTP-compliant
//! status codes and messages.

use crate::protocol::CommandStatus;
use log::{error, info, warn};
use std::fs::{File, remove_file, rename};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

const MAX_RETRIES: usize = 3;
const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB in bytes
const BUFFER_SIZE: usize = 8192; // 8KB buffer for better performance

/// Handles uploading a file from the client to the server using temporary files.
///
/// This function implements atomic file uploads by writing to a temporary file first,
/// then renaming it to the final destination on successful completion.
pub fn handle_file_upload(
    mut data_stream: TcpStream,
    final_filename: &str,
    temp_filename: &str,
) -> Result<(CommandStatus, &'static str), (CommandStatus, &'static str)> {
    info!(
        "Starting file upload: {temp_filename} -> {final_filename}"
    );

    // Create temporary file for atomic upload
    let mut temp_file = match File::create(temp_filename) {
        Ok(file) => file,
        Err(e) => {
            error!("Failed to create temporary file {temp_filename}: {e}");
            return Err((
                CommandStatus::Failure("550 Cannot create file".into()),
                "550 Cannot create file\r\n",
            ));
        }
    };

    let mut buffer = [0; BUFFER_SIZE];
    let mut total_bytes_received = 0u64;

    // Send initial response indicating data transfer is starting
    info!("Ready to receive data for {final_filename}");

    loop {
        let mut retries = 0;
        let n = loop {
            match data_stream.read(&mut buffer) {
                Ok(0) => break 0, // EOF - upload complete
                Ok(n) => break n,
                Err(e) if retries < MAX_RETRIES => {
                    warn!(
                        "Transient read error (attempt {}/{}): {}. Retrying...",
                        retries + 1,
                        MAX_RETRIES,
                        e
                    );
                    retries += 1;
                    thread::sleep(Duration::from_millis(100 * retries as u64));
                }
                Err(e) => {
                    error!("Read failure after {MAX_RETRIES} retries: {e}");
                    // Clean up temporary file
                    let _ = remove_file(temp_filename);
                    return Err((
                        CommandStatus::Failure("426 Connection closed; transfer aborted".into()),
                        "426 Connection closed; transfer aborted\r\n",
                    ));
                }
            }
        };

        if n == 0 {
            break; // End of file reached
        }

        // Check file size limit BEFORE writing (fail fast)
        total_bytes_received += n as u64;
        if total_bytes_received > MAX_FILE_SIZE {
            error!(
                "File size limit exceeded: {total_bytes_received} bytes > {MAX_FILE_SIZE} bytes (100MB)"
            );
            // Clean up temporary file
            let _ = remove_file(temp_filename);
            return Err((
                CommandStatus::Failure("552 Insufficient storage space".into()),
                "552 Insufficient storage space (file too large, max 100MB)\r\n",
            ));
        }

        // Write chunk to temporary file
        if let Err(e) = temp_file.write_all(&buffer[..n]) {
            error!("Failed to write to temporary file {temp_filename}: {e}");
            // Clean up temporary file
            let _ = remove_file(temp_filename);
            return Err((
                CommandStatus::Failure("552 Insufficient storage space".into()),
                "552 Insufficient storage space\r\n",
            ));
        }
    }

    // Ensure all data is written to disk
    if let Err(e) = temp_file.flush() {
        error!("Failed to flush temporary file {temp_filename}: {e}");
        let _ = remove_file(temp_filename);
        return Err((
            CommandStatus::Failure("450 Requested file action not taken".into()),
            "450 Requested file action not taken\r\n",
        ));
    }

    // Explicitly close the temporary file
    drop(temp_file);

    // Atomically move temporary file to final location
    match rename(temp_filename, final_filename) {
        Ok(_) => {
            info!(
                "File upload completed successfully: {final_filename} ({total_bytes_received} bytes)"
            );
            Ok((CommandStatus::Success, "226 Transfer complete\r\n"))
        }
        Err(e) => {
            error!(
                "Failed to rename {temp_filename} to {final_filename}: {e}"
            );
            // Clean up temporary file if rename failed
            let _ = remove_file(temp_filename);
            Err((
                CommandStatus::Failure("450 Requested file action not taken".into()),
                "450 Requested file action not taken\r\n",
            ))
        }
    }
}

/// Handles downloading a file from the server to the client.
pub fn handle_file_download(
    mut data_stream: TcpStream,
    filename: &str,
) -> Result<(CommandStatus, &'static str), (CommandStatus, &'static str)> {
    info!("Starting file download: {filename}");

    let mut file = match File::open(filename) {
        Ok(file) => file,
        Err(e) => {
            error!("Failed to open file {filename}: {e}");
            return Err((
                CommandStatus::Failure("550 Failed to open file".into()),
                "550 Failed to open file\r\n",
            ));
        }
    };

    let mut buffer = [0; BUFFER_SIZE];
    let mut total_bytes_sent = 0u64;

    loop {
        let n = match file.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => n,
            Err(e) => {
                error!("Read error on {filename}: {e}");
                return Err((
                    CommandStatus::Failure("451 Requested action aborted".into()),
                    "451 Requested action aborted\r\n",
                ));
            }
        };

        let mut retries = 0;
        loop {
            match data_stream.write_all(&buffer[..n]) {
                Ok(_) => break,
                Err(e) if retries < MAX_RETRIES => {
                    warn!(
                        "Transient write error (attempt {}/{}): {}. Retrying...",
                        retries + 1,
                        MAX_RETRIES,
                        e
                    );
                    retries += 1;
                    thread::sleep(Duration::from_millis(100 * retries as u64));
                }
                Err(e) => {
                    error!(
                        "Write failure to data stream after {MAX_RETRIES} retries: {e}"
                    );
                    return Err((
                        CommandStatus::Failure("426 Connection closed; transfer aborted".into()),
                        "426 Connection closed; transfer aborted\r\n",
                    ));
                }
            }
        }

        total_bytes_sent += n as u64;
    }

    if let Err(e) = data_stream.flush() {
        error!("Failed to flush data stream: {e}");
        return Err((
            CommandStatus::Failure("450 Requested file action not taken".into()),
            "450 Requested file action not taken\r\n",
        ));
    }

    info!(
        "File download completed successfully: {filename} ({total_bytes_sent} bytes)"
    );

    Ok((CommandStatus::Success, "226 Transfer complete\r\n"))
}
