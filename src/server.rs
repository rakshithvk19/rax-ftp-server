use log::{error, info, warn};
use std::collections::HashMap;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::client::Client;
use crate::client_handler::handle_client;

pub struct Server {
    clients: Arc<Mutex<HashMap<SocketAddr, Client>>>,
}

const COMMAND_SOCKET: &str = "127.0.0.1:2121";
const MAX_CLIENTS: usize = 10;

impl Server {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn check_max_clients(&self, cmd_stream: &mut TcpStream, client_addr: &SocketAddr) -> bool {
        let client_count = {
            let clients = self.clients.lock().unwrap();
            clients.len()
        };

        if client_count >= MAX_CLIENTS {
            warn!(
                "Max clients ({}) reached. Rejecting connection from {}",
                MAX_CLIENTS, client_addr
            );
            let _ = cmd_stream.write_all(b"421 Too many connections. Server busy.\r\n");
            let _ = cmd_stream.flush();
            true
        } else {
            false
        }
    }

    pub fn start(&self) {
        info!(
            "Starting Rax FTP server on {} (max {} clients)",
            COMMAND_SOCKET, MAX_CLIENTS
        );
        let listener = TcpListener::bind(COMMAND_SOCKET).unwrap();

        for cmd_stream in listener.incoming() {
            match cmd_stream {
                Ok(mut cmd_stream) => {
                    // let client_addr = cmd_stream.peer_addr().unwrap().to_string();

                    let client_addr = match cmd_stream.peer_addr() {
                        Ok(addr) => addr,
                        Err(e) => {
                            error!("Failed to get peer address: {}", e);
                            // Drop the stream and continue accepting other connections

                            drop(cmd_stream);
                            continue;
                        }
                    };

                    //check for max clients and drop cmd_stream if exceeding
                    if self.check_max_clients(&mut cmd_stream, &client_addr) {
                        drop(cmd_stream);
                        continue;
                    }

                    //Logging client connections
                    info!(
                        "New connection: {} ({}/{} clients)",
                        client_addr,
                        {
                            let clients = self.clients.lock().unwrap();
                            clients.len() + 1
                        },
                        MAX_CLIENTS
                    );

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
