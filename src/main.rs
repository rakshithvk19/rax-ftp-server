// src/main.rs
mod server;
use log::{error, info};
use server::handle_client;
use std::net::TcpListener;

fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting FTP server on 127.0.0.1:2121");

    let listener = TcpListener::bind("127.0.0.1:2121")?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                info!("New connection: {}", stream.peer_addr()?);
                std::thread::spawn(|| {
                    handle_client(stream);
                });
            }
            Err(e) => error!("Error accepting connection: {}", e),
        }
    }
    Ok(())
}
