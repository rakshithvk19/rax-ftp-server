//! RAX FTP Server - Entry Point
//!
//! A robust Rust-based FTP server implementing core features of RFC 959.

use env_logger;
use log::info;

mod auth;
mod client;
mod error;
mod navigate;
mod protocol;
mod server;
mod storage;
mod transfer;

use server::Server;

#[tokio::main]
async fn main() {
    // Initialize the logger (env_logger picks up RUST_LOG environment variable)
    env_logger::init();

    info!("Launching Rax FTP server...");

    let server = Server::new().await;
    server.start().await;
}
