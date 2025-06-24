use log::{error, info};

use rax_ftp_server::Server;

fn main() {
    env_logger::init();

    info!("Starting Rax FTP Server...");
    let server = Server::new();

    if let Err(e) = std::panic::catch_unwind(|| server.start()) {
        error!("Rax FTP Server crashed: {:?}", e);
        std::process::exit(1);
    } else {
        info!("Rax FTP Server shutdown gracefully");
    }
}
