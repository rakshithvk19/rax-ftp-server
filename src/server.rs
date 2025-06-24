// server.rs
// This file implements the main FTP server, managing client connections,
// enforcing a client limit, and spawning handler threads for each new client.

use log::{error, info, warn};
use std::collections::HashMap;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::channel_registry::ChannelRegistry;
use crate::client::Client;
use crate::client_handler::handle_client;

pub struct Server {
    client_registry: Arc<Mutex<HashMap<SocketAddr, Client>>>,
    channel_registry: Arc<Mutex<ChannelRegistry>>,
}

const COMMAND_SOCKET: &str = "127.0.0.1:2121";
const MAX_CLIENTS: usize = 10;

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        Self {
            client_registry: Arc::new(Mutex::new(HashMap::new())),
            channel_registry: Arc::new(Mutex::new(ChannelRegistry::default())),
        }
    }

    fn check_max_clients(&self, cmd_stream: &mut TcpStream, client_addr: &SocketAddr) -> bool {
        let client_count = {
            let clients = self.client_registry.lock().unwrap();
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
                            let clients = self.client_registry.lock().unwrap();
                            clients.len() + 1
                        },
                        MAX_CLIENTS
                    );

                    {
                        let mut clients = self.client_registry.lock().unwrap();
                        let mut client = Client::default();
                        client.set_client_addr(Some(client_addr));
                        clients.insert(client_addr, client);
                    }

                    let client_registry_ref = Arc::clone(&self.client_registry);
                    let channel_registry_ref = Arc::clone(&self.channel_registry);

                    thread::spawn(move || {
                        handle_client(
                            cmd_stream,
                            client_registry_ref,
                            client_addr,
                            channel_registry_ref,
                        );
                    });
                }
                Err(e) => error!("Error accepting connection: {}", e),
            }
        }
    }
}
