pub mod auth;
pub mod channel_registry;
pub mod client;
pub mod client_handler;
pub mod command;
pub mod data_channel;
pub mod file_transfer;
pub mod handlers;
mod server;

pub fn start_server() {
    let server = server::Server::new();
    server.start();
}
