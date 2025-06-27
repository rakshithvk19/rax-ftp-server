use env_logger;
use log::info;
use rax_ftp_server::server::Server;

#[tokio::main]
async fn main() {
    // Initialize the logger (env_logger picks up RUST_LOG environment variable)
    env_logger::init();

    info!("Launching FTP server...");

    let server = Server::new().await;
    server.start().await;
}
