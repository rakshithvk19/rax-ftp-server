// server.rs

use log::{error, info, warn};
use std::collections::HashMap;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::channel_registry::ChannelRegistry;
use crate::client::Client;
use crate::client_handler::handle_client;

/// Main FTP Server struct responsible for accepting incoming client connections,
/// managing connected clients and data channels, and delegating client handling
/// to worker threads.
///
/// The server listens on a TCP socket and maintains registries for active clients
/// and data channels. It enforces a maximum concurrent client limit.
pub(crate) struct Server {
    /// Registry mapping client socket addresses to Client state objects.
    client_registry: Arc<Mutex<HashMap<SocketAddr, Client>>>,

    /// Registry managing data channels associated with clients.
    channel_registry: Arc<Mutex<ChannelRegistry>>,

    /// TCP listener socket bound to the command port to accept new client connections.
    listener: TcpListener,
}

/// Default address and port where the FTP command socket listens.
const COMMAND_SOCKET: &str = "127.0.0.1:2121";

/// Maximum number of concurrent client connections allowed.
const MAX_CLIENTS: usize = 10;

impl Default for Server {
    /// Default implementation simply calls `new()` constructor.
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    /// Constructs a new Server instance by binding the command socket
    /// and initializing empty registries for clients and data channels.
    ///
    /// # Panics
    ///
    /// Panics if binding to the command socket address fails.
    pub fn new() -> Self {
        let listener = TcpListener::bind(COMMAND_SOCKET).expect("Failed to bind to command socket");

        Self {
            client_registry: Arc::new(Mutex::new(HashMap::new())),
            channel_registry: Arc::new(Mutex::new(ChannelRegistry::default())),
            listener,
        }
    }

    /// Provides a thread-safe reference-counted clone of the client registry.
    pub fn client_registry(&self) -> Arc<Mutex<HashMap<SocketAddr, Client>>> {
        Arc::clone(&self.client_registry)
    }

    /// Provides a thread-safe reference-counted clone of the channel registry.
    pub fn channel_registry(&self) -> Arc<Mutex<ChannelRegistry>> {
        Arc::clone(&self.channel_registry)
    }

    /// Returns a reference to the bound TCP listener socket.
    pub fn listener(&self) -> &TcpListener {
        &self.listener
    }

    /// Starts the FTP server event loop, accepting client connections
    /// and delegating client handling to worker threads.
    ///
    /// Logs startup information and continuously listens for incoming connections.
    pub fn start(&self) {
        info!(
            "Starting Rax FTP server on {} (max {} clients)",
            COMMAND_SOCKET, MAX_CLIENTS
        );
        self.accept_client(&self.listener);
    }

    /// Accepts incoming client connections on the provided TCP listener.
    ///
    /// For each accepted connection:
    /// - Obtains the client's socket address.
    /// - Checks if the maximum client limit is exceeded, rejecting if necessary.
    /// - Registers the client.
    /// - Spawns a dedicated thread to handle client commands.
    ///
    /// Logs errors on connection acceptance failures.
    fn accept_client(&self, listener: &TcpListener) {
        for cmd_stream in listener.incoming() {
            match cmd_stream {
                Ok(mut cmd_stream) => {
                    // Extract client's socket address
                    if let Some(client_addr) = self.get_client_address(&mut cmd_stream) {
                        // Reject connection if max clients reached
                        if self.check_max_clients(&mut cmd_stream, &client_addr) {
                            continue;
                        }

                        // Register new client in client registry
                        self.register_client(client_addr);

                        // Spawn a new thread to handle client commands asynchronously
                        self.spawn_handler(cmd_stream, client_addr);
                    }
                }
                Err(e) => error!("Error accepting connection: {}", e),
            }
        }
    }

    /// Retrieves the remote socket address of the connected client from the TCP stream.
    ///
    /// Logs and returns None if the peer address cannot be obtained.
    fn get_client_address(&self, cmd_stream: &mut TcpStream) -> Option<SocketAddr> {
        match cmd_stream.peer_addr() {
            Ok(addr) => Some(addr),
            Err(e) => {
                error!("Failed to get peer address: {}", e);
                None
            }
        }
    }

    /// Checks if the current number of connected clients exceeds the maximum allowed.
    ///
    /// If the limit is reached:
    /// - Sends a "421 Too many connections" response to the client.
    /// - Flushes the stream.
    /// - Returns true indicating the connection should be rejected.
    ///
    /// Otherwise returns false allowing the connection to proceed.
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

    /// Registers a new client in the client registry with the given socket address.
    ///
    /// Initializes a default Client instance and stores it in the registry.
    /// Logs the new connection and current client count.
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

    /// Spawns a new thread to asynchronously handle the connected client.
    ///
    /// The handler thread receives ownership of the command stream,
    /// client registry, client address, and channel registry for
    /// managing client interactions and FTP data channels.
    fn spawn_handler(&self, cmd_stream: TcpStream, client_addr: SocketAddr) {
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
}
