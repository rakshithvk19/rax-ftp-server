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

const COMMAND_SOCKET: &str = "127.0.0.1:2121";
const MAX_CLIENTS: usize = 10;

pub struct Server {
    client_registry: Arc<Mutex<HashMap<SocketAddr, Client>>>,
    channel_registry: Arc<Mutex<ChannelRegistry>>,
    listener: TcpListener,
}

impl Server {
    pub async fn new() -> Self {
        let listener = match TcpListener::bind(COMMAND_SOCKET).await {
            Ok(listener) => {
                info!("Server bound to {}", COMMAND_SOCKET);
                listener
            }
            Err(e) => {
                error!("Failed to bind to {}: {}", COMMAND_SOCKET, e);
                panic!("Server startup failed on socket {}: {}", COMMAND_SOCKET, e);
            }
        };

        Self {
            client_registry: Arc::new(Mutex::new(HashMap::new())),
            channel_registry: Arc::new(Mutex::new(ChannelRegistry::default())),
            listener,
        }
    }

    pub async fn start(&self) {
        info!(
            "Starting Rax FTP server on {} (max {} clients)",
            COMMAND_SOCKET, MAX_CLIENTS
        );

        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    let client_registry = Arc::clone(&self.client_registry);
                    let channel_registry = Arc::clone(&self.channel_registry);

                    // Spawn a task for each client so accept loop doesn't block
                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_new_client(stream, addr, client_registry, channel_registry).await
                        {
                            warn!("Failed to handle client {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
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
) -> Result<(), std::io::Error> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    // Send greeting
    reader
        .get_mut()
        .write_all(b"220 Welcome to RAX FTP Server\r\n")
        .await?;

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
        let result = handle_auth_command(&mut client, &command);

        if let Some(msg) = result.message {
            reader.get_mut().write_all(msg.as_bytes()).await?;
        }

        if client.is_logged_in() {
            let mut clients = client_registry.lock().await;

            if clients.len() >= MAX_CLIENTS {
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
                MAX_CLIENTS
            );

            let cmd_stream = reader.into_inner();

            drop(clients);

            // Hand off to session handler
            handle_client(cmd_stream, client_registry, client_addr, channel_registry).await;

            return Ok(());
        }
    }
}
