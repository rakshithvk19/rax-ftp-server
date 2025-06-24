//file_transfer.rs

use crate::command::CommandStatus;
use log::error;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;

pub fn handle_file_upload(
    mut data_stream: TcpStream,
    filename: &str,
) -> Result<(CommandStatus, &'static str), (CommandStatus, &'static str)> {
    match File::create(filename) {
        Ok(mut file) => {
            let mut buffer = [0; 1024];
            loop {
                match data_stream.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
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
                            CommandStatus::Failure(
                                "426 Connection closed; transfer aborted\r\n".into(),
                            ),
                            "426 Connection closed; transfer aborted\r\n",
                        ));
                    }
                }
            }

            if file.flush().is_ok() {
                return Ok((CommandStatus::Success, "226 Transfer complete\r\n"));
            } else {
                return Err((
                    CommandStatus::Failure("450 Requested file action not taken\r\n".into()),
                    "450 Requested file action not taken\r\n",
                ));
            }
        }
        Err(e) => {
            error!("Failed to create file {}: {}", filename, e);
            return Err((
                CommandStatus::Failure("550 Requested action not taken\r\n".into()),
                "550 Requested action not taken\r\n",
            ));
        }
    }
}

pub fn handle_file_download(
    mut data_stream: TcpStream,
    filename: &str,
) -> Result<(CommandStatus, &'static str), (CommandStatus, &'static str)> {
    match File::open(filename) {
        Ok(mut file) => {
            let mut buffer = [0; 1024];

            loop {
                match file.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if let Err(e) = data_stream.write_all(&buffer[..n]) {
                            error!("Failed to write to data stream: {}", e);
                            return Err((
                                CommandStatus::Failure(
                                    "426 Connection closed; transfer aborted\r\n".into(),
                                ),
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
