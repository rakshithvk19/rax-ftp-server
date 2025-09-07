use log::{error, info, warn};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::client::Client;
use crate::client::handle_client;
use crate::config::{ServerConfig, SharedRuntimeConfig, StartupConfig};
use crate::protocol::handle_auth_command;
use crate::protocol::parse_command;
use crate::transfer::ChannelRegistry;

pub struct Server {
    client_registry: Arc<Mutex<HashMap<SocketAddr, Client>>>,
    channel_registry: Arc<Mutex<ChannelRegistry>>,
    listener: TcpListener,
    startup_config: Arc<StartupConfig>,
    runtime_config: SharedRuntimeConfig,
}

impl Server {
    pub async fn new() -> Self {
        // Load configuration from config.toml and environment
        let config = ServerConfig::load().expect("Failed to load server configuration");
        let (startup_config, runtime_config) = config.split();

        let startup_config = Arc::new(startup_config);

        let listener = match TcpListener::bind(&startup_config.control_socket()).await {
            Ok(listener) => {
                info!("Server bound to {}", startup_config.control_socket());
                listener
            }
            Err(e) => {
                error!("Failed to bind to {}: {e}", startup_config.control_socket());
                panic!(
                    "Server startup failed on socket {}: {e}",
                    startup_config.control_socket()
                );
            }
        };

        // Ensure server root directory exists
        if let Err(e) = std::fs::create_dir_all(startup_config.server_root_path()) {
            warn!("Failed to create server root directory: {e}");
        } else {
            info!(
                "Server root directory: {}",
                startup_config.server_root_str()
            );
        }

        Self {
            client_registry: Arc::new(Mutex::new(HashMap::new())),
            channel_registry: Arc::new(Mutex::new(ChannelRegistry::default())),
            listener,
            startup_config,
            runtime_config,
        }
    }

    pub async fn start(&self) {
        let runtime_config = self.runtime_config.read().await;
        info!(
            "Starting Rax FTP server on {} (max {} clients)",
            self.startup_config.control_socket(),
            runtime_config.max_clients
        );
        drop(runtime_config);

        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    info!("Client {addr} connected to FTP server");
                    let client_registry = Arc::clone(&self.client_registry);
                    let channel_registry = Arc::clone(&self.channel_registry);
                    let startup_config = Arc::clone(&self.startup_config);
                    let runtime_config = Arc::clone(&self.runtime_config);

                    // Spawn a task for each client so accept loop doesn't block
                    tokio::spawn(async move {
                        if let Err(e) = handle_new_client(
                            stream,
                            addr,
                            client_registry,
                            channel_registry,
                            startup_config,
                            runtime_config,
                        )
                        .await
                        {
                            warn!("Failed to handle client {addr}: {e}");
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting connection: {e}");
                }
            }
        }
    }
}

/// Handles a new client: greets, authenticates, registers, and spawns session handler.
async fn handle_new_client(
    stream: TcpStream,
    client_addr: SocketAddr,
    client_registry: Arc<Mutex<HashMap<SocketAddr, Client>>>,
    channel_registry: Arc<Mutex<ChannelRegistry>>,
    startup_config: Arc<StartupConfig>,
    runtime_config: SharedRuntimeConfig,
) -> Result<(), std::io::Error> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    // Send greeting
    reader
        .get_mut()
        .write_all(b"220 Welcome to RAX FTP Server\r\n")
        .await?;

    // FLUSH THE GREETING MESSAGE IMMEDIATELY
    reader.get_mut().flush().await?;

    let mut client = Client::default();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "Client disconnected during authentication",
            ));
        }

        let command = parse_command(&line);
        let result = handle_auth_command(&mut client, &command, &startup_config);

        if let Some(msg) = result.message {
            reader.get_mut().write_all(msg.as_bytes()).await?;
        }

        if client.is_logged_in() {
            let mut clients = client_registry.lock().await;
            let runtime = runtime_config.read().await;

            if clients.len() >= runtime.max_clients {
                reader
                    .get_mut()
                    .write_all(b"421 Too many connections. Try again later.\r\n")
                    .await?;
                return Ok(()); // Close connection
            }

            client.set_client_addr(Some(client_addr));
            clients.insert(client_addr, client);

            info!(
                "Authenticated client: {} ({}/{} clients)",
                client_addr,
                clients.len(),
                runtime.max_clients
            );

            let cmd_stream = reader.into_inner();

            drop(clients);
            drop(runtime);

            // Hand off to session handler
            handle_client(
                cmd_stream,
                client_registry,
                client_addr,
                channel_registry,
                startup_config,
                runtime_config,
            )
            .await;

            return Ok(());
        }
    }
}
