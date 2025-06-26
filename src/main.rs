// main.rs
// Entry point for the Rax FTP Server application.
// Initializes logging and starts the FTP server.

use rax_ftp_server::start_server;

fn main() {
    // Initialize the logger from environment variables (e.g., RUST_LOG)
    // This enables logging throughout the application, useful for debugging and monitoring.
    env_logger::init();

    // Start the FTP server event loop
    start_server();
}
