use log::info;
use std::io;

use rax_ftp_server::start_server;

fn main() -> io::Result<()> {
    env_logger::init();
    info!("Initializing FTP server");
    start_server("127.0.0.1:2121")
}