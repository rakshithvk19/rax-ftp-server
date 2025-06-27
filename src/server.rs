// server.rs
use log::{error, info, warn};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::channel_registry::ChannelRegistry;
use crate::client::Client;
use crate::client_handler::handle_client;
use crate::command::parse_command;
use crate::handlers::handle_auth_command;

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

impl Server {
    /// Creates a new FTP server instance and binds the TCP listener.
    /// Logs detailed error and panics on failure to bind.
    /// Binds TCP listener to the specified command socket address.
    pub async fn new() -> Self {
        let listener = match TcpListener::bind(COMMAND_SOCKET).await {
            Ok(listener) => {
                info!("Server bound to {}", COMMAND_SOCKET);
                listener
            }
            Err(e) => {
                error!("Failed to bind to {}: {}", COMMAND_SOCKET, e);
                panic!(
                    "Server startup failed on socket {} due to : {}",
                    COMMAND_SOCKET, e
                );
            }
        };

        Self {
            client_registry: Arc::new(Mutex::new(HashMap::new())),
            channel_registry: Arc::new(Mutex::new(ChannelRegistry::default())),
            listener,
        }
    }

    async fn accept_clients(&self) {
        loop {
            match self.listener.accept().await {
                Ok((cmd_stream, client_addr)) => {
                    // Check if the client limit is reached
                    let client_count = {
                        let clients = self.client_registry.lock().await;
                        clients.len()
                    };

                    if client_count >= MAX_CLIENTS {
                        warn!(
                            "Max clients ({}) reached. Rejecting connection from {}",
                            MAX_CLIENTS, client_addr
                        );
                        let mut stream = cmd_stream;
                        let _ = stream
                            .write_all(b"421 Too many connections. Server busy.\r\n")
                            .await;
                        let _ = stream.flush().await;
                        continue;
                    }

                    // Authenticate client and register on success
                    match self
                        .authenticate_and_register_client(cmd_stream, client_addr)
                        .await
                    {
                        Ok(stream) => {
                            self.spawn_handler(stream, client_addr);
                        }
                        Err(e) => {
                            warn!("Authentication failed for {}: {}", client_addr, e);
                            // Connection will be dropped here, closing stream
                            continue;
                        }
                    }
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }
    }

    /// Authenticate client using line-based reading and register client upon success.
    ///
    /// Returns the TcpStream to continue handling after authentication.
    async fn authenticate_and_register_client(
        &self,
        stream: TcpStream,
        client_addr: SocketAddr,
    ) -> Result<TcpStream, std::io::Error> {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();

        let mut client = Client::default();

        reader
            .get_mut()
            .write_all(b"220 Welcome to RAX FTP\r\n")
            .await?;

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "Client disconnected during auth",
                ));
            }

            let cmd = parse_command(&line);
            let res = handle_auth_command(&mut client, &cmd);

            if let Some(msg) = res.message {
                reader.get_mut().write_all(msg.as_bytes()).await?;
            }

            if client.is_logged_in() {
                // Register the client in the registry now that authentication succeeded
                {
                    let mut clients = self.client_registry.lock().await;
                    client.set_client_addr(Some(client_addr));
                    clients.insert(client_addr, client);
                    info!(
                        "New authenticated client: {} ({}/{} clients)",
                        client_addr,
                        clients.len(),
                        MAX_CLIENTS
                    );
                }

                // Extract the TcpStream from BufReader to pass on
                let stream = reader.into_inner();
                break Ok(stream);
            }
        }
    }

    // TODO: Call this method on QUIT command or disconnect
    async fn deregister_client(&self, client_addr: SocketAddr) {
        let mut clients = self.client_registry.lock().await;
        clients.remove(&client_addr);
    }

    /// Spawns an async task to manage the client's FTP session.
    /// Logs and removes the client from the registry if the task panics.
    fn spawn_handler(&self, cmd_stream: TcpStream, client_addr: SocketAddr) {
        let client_registry_ref = Arc::clone(&self.client_registry);
        let channel_registry_ref = Arc::clone(&self.channel_registry);

        tokio::spawn(async move {
            handle_client(
                cmd_stream,
                client_registry_ref,
                client_addr,
                channel_registry_ref,
            )
            .await;
        });
    }

    pub async fn start(&self) {
        info!(
            "Starting Rax FTP server on {} (max {} clients)",
            COMMAND_SOCKET, MAX_CLIENTS
        );
        self.accept_clients().await;
    }
}
