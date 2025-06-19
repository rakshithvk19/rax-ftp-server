use log::{error, info};
use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::client::Client;
use crate::client_handler::handle_client;

pub struct Server {
    clients: Arc<Mutex<HashMap<String, Client>>>,
}

const COMMAND_SOCKET: &str = "127.0.0.1:2121";

impl Server {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(&self) {
        info!("Starting Rax FTP server on {}", COMMAND_SOCKET);
        let listener = TcpListener::bind(COMMAND_SOCKET).unwrap();

        for cmd_stream in listener.incoming() {
            match cmd_stream {
                Ok(cmd_stream) => {
                    let client_addr = cmd_stream.peer_addr().unwrap().to_string();
                    
                    info!("New connection: {}", client_addr);

                    {
                        let mut clients = self.clients.lock().unwrap();
                        clients.insert(client_addr.clone(), Client::default());
                    }

                    let clients_ref = Arc::clone(&self.clients);

                    thread::spawn(move || {
                        handle_client(cmd_stream, clients_ref, client_addr);
                    });
                }
                Err(e) => error!("Error accepting connection: {}", e),
            }
        }
    }
}
