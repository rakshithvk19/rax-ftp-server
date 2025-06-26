// server.rs
use log::{error, info, warn};
use std::collections::HashMap;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::channel_registry::ChannelRegistry;
use crate::client::Client;
use crate::client_handler::handle_client;

/// The main FTP server struct that manages TCP listener, client registry,
/// and channel registry. It accepts incoming connections and spawns
/// handler threads to process FTP client sessions.
pub struct Server {
    client_registry: Arc<Mutex<HashMap<SocketAddr, Client>>>,
    channel_registry: Arc<Mutex<ChannelRegistry>>,
    listener: TcpListener,
}

/// Default control socket address for FTP command communication.
const COMMAND_SOCKET: &str = "127.0.0.1:2121";

/// Maximum number of clients that can connect simultaneously.
const MAX_CLIENTS: usize = 10;

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    /// Creates a new FTP server instance and binds the TCP listener.
    /// Logs detailed error and panics on failure to bind.
    pub fn new() -> Self {
        let listener = TcpListener::bind(COMMAND_SOCKET).unwrap_or_else(|e| {
            error!("Failed to bind to {}: {}", COMMAND_SOCKET, e);
            panic!("Server startup failed: {}", e);
        });

        Self {
            client_registry: Arc::new(Mutex::new(HashMap::new())),
            channel_registry: Arc::new(Mutex::new(ChannelRegistry::default())),
            listener,
        }
    }

    /// Accepts and manages incoming client connections.
    /// Applies read timeout and handles client capacity limits.
    fn accept_clients(&self) {
        for cmd_stream in self.listener.incoming() {
            match cmd_stream {
                Ok(mut cmd_stream) => {
                    // Set timeout to avoid idle clients consuming resources
                    cmd_stream
                        .set_read_timeout(Some(Duration::from_secs(300)))
                        .unwrap_or_else(|e| error!("Failed to set read timeout: {}", e));

                    if let Some(client_addr) = self.get_client_address(&mut cmd_stream) {
                        if self.check_max_clients(&mut cmd_stream, &client_addr) {
                            continue;
                        }

                        self.register_client(client_addr);
                        self.spawn_handler(cmd_stream, client_addr);
                    }
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                    thread::sleep(Duration::from_millis(100)); // Prevent tight accept loop
                }
            }
        }
    }

    /// Retrieves the client's socket address from the TCP stream.
    fn get_client_address(&self, cmd_stream: &mut TcpStream) -> Option<SocketAddr> {
        match cmd_stream.peer_addr() {
            Ok(addr) => Some(addr),
            Err(e) => {
                error!("Failed to get peer address: {}", e);
                None
            }
        }
    }

    /// Checks if the client limit is reached.
    /// If exceeded, sends a message and denies the connection.
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

    /// Registers the client into the shared registry with initial state.
    fn register_client(&self, client_addr: SocketAddr) {
        info!(
            "New connection: {} ({}/{} clients)",
            client_addr,
            {
                let clients = self.client_registry.lock().unwrap();
                clients.len() + 1
            },
            MAX_CLIENTS
        );

        let mut clients = self.client_registry.lock().unwrap();
        let mut client = Client::default();
        client.set_client_addr(Some(client_addr));
        clients.insert(client_addr, client);
    }

    /// Spawns a handler thread to manage the client's FTP session.
    /// Logs and cleans up if thread creation fails.
    fn spawn_handler(&self, cmd_stream: TcpStream, client_addr: SocketAddr) {
        let client_registry_ref = Arc::clone(&self.client_registry);
        let channel_registry_ref = Arc::clone(&self.channel_registry);

        if let Err(e) = thread::Builder::new()
            .name(format!("client-handler-{}", client_addr))
            .spawn(move || {
                handle_client(
                    cmd_stream,
                    client_registry_ref,
                    client_addr,
                    channel_registry_ref,
                );
            })
        {
            error!("Failed to spawn thread for client {}: {}", client_addr, e);
            let mut clients = self.client_registry.lock().unwrap();
            clients.remove(&client_addr);
        }
    }

    /// Starts the server by listening for incoming FTP connections.
    pub fn start(&self) {
        info!(
            "Starting Rax FTP server on {} (max {} clients)",
            COMMAND_SOCKET, MAX_CLIENTS
        );
        self.accept_clients();
    }
}
