//! Module `channel_registry`
//!
//! Provides a centralized registry to manage FTP data channels per client,
//! including active data sockets, TCP streams, and passive-mode listeners.
//! Facilitates allocation and lifecycle management of data connections used
//! for file transfers (e.g., STOR, RETR, LIST).
//! Updated to support persistent data connections.

use log::warn;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};

/// Represents the state of a single FTP data channel associated with a client.
/// Contains optional references to the client's data socket address,
/// the active data stream, any passive mode listener, and client ownership info.
#[derive(Default)]
pub struct ChannelEntry {
    data_socket: Option<SocketAddr>, // IP:Port the client uses for active data connection
    data_stream: Option<TcpStream>,  // Established TCP stream for the data transfer
    listener: Option<TcpListener>,   // Listener socket for passive mode connections
    owner_ip: Option<IpAddr>,        // IP address of the client that owns this channel
}

impl ChannelEntry {
    // --- Accessors ---

    /// Returns a reference to the data socket address if present.
    pub fn data_socket(&self) -> Option<&SocketAddr> {
        self.data_socket.as_ref()
    }

    /// Returns a reference to the passive mode TCP listener if present.
    pub fn listener(&self) -> Option<&TcpListener> {
        self.listener.as_ref()
    }

    /// Returns a mutable reference to the passive mode listener if present.
    pub fn listener_mut(&mut self) -> Option<&mut TcpListener> {
        self.listener.as_mut()
    }

    // --- Setters ---

    /// Sets the data socket address, replacing any existing value.
    pub fn set_data_socket(&mut self, socket: Option<SocketAddr>) {
        self.data_socket = socket;
    }

    /// Sets the data TCP stream, replacing any existing value.
    pub fn set_data_stream(&mut self, stream: Option<TcpStream>) {
        self.data_stream = stream;
    }

    /// Sets the passive mode TCP listener, replacing any existing value.
    pub fn set_listener(&mut self, listener: Option<TcpListener>) {
        self.listener = listener;
    }

    /// Sets the owner IP address for this channel.
    pub fn set_owner_ip(&mut self, ip: Option<IpAddr>) {
        self.owner_ip = ip;
    }

    /// Cleans up only the data stream, keeping the persistent setup intact.
    pub fn cleanup_stream_only(&mut self) {
        if let Some(stream) = self.data_stream.take() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }

    /// Completely cleans up all resources in this entry.
    pub fn cleanup_all(&mut self) {
        self.cleanup_stream_only();
        self.listener = None;
        self.data_socket = None;
        self.owner_ip = None;
    }
}

/// Registry that maps client socket addresses to their corresponding FTP data channels.
/// Manages allocation and bookkeeping of active data connections with persistent support.
#[derive(Default)]
pub struct ChannelRegistry {
    registry: HashMap<SocketAddr, ChannelEntry>,
}

impl ChannelRegistry {
    /// Port range used for PASV (passive) mode data channel listeners.
    /// The server listens on these ports to accept incoming client data connections.
    pub const DATA_PORT_RANGE: std::ops::Range<u16> = 2122..2222;

    /// Inserts or replaces the data channel entry associated with the given client address.
    ///
    /// If the provided data socket is already in use by another client, it logs a warning and skips insertion.
    pub fn insert(&mut self, addr: SocketAddr, entry: ChannelEntry) {
        if let Some(socket) = entry.data_socket {
            if self.is_socket_taken(&socket) {
                warn!("Attempted to insert a data socket already in use: {socket}");
                return;
            }
        }
        self.registry.insert(addr, entry);
    }

    /// Removes and returns the data channel entry for a given client address, if any.
    pub fn remove(&mut self, addr: &SocketAddr) -> Option<ChannelEntry> {
        self.registry.remove(addr)
    }

    /// Returns a mutable reference to the data channel entry for a client address, if present.
    pub fn get_mut(&mut self, addr: &SocketAddr) -> Option<&mut ChannelEntry> {
        self.registry.get_mut(addr)
    }

    /// Checks whether a data channel entry exists for the given client address.
    pub fn contains(&self, addr: &SocketAddr) -> bool {
        self.registry.contains_key(addr)
    }

    /// Attempts to find the next available socket address in the configured PASV port range
    /// that is not currently assigned to any client's data socket.
    pub fn next_available_socket(&self) -> Option<SocketAddr> {
        for port in Self::DATA_PORT_RANGE {
            let data_socket: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
            if !self.is_socket_taken(&data_socket) {
                return Some(data_socket);
            }
        }
        None
    }

    /// Checks if the given socket address is already assigned as a data socket for any client.
    pub fn is_socket_taken(&self, addr: &SocketAddr) -> bool {
        self.registry
            .values()
            .any(|entry| entry.data_socket.as_ref() == Some(addr))
    }

    /// Completely cleans up all data channel resources for a client.
    pub fn cleanup_all(&mut self, client_addr: &SocketAddr) {
        if let Some(mut entry) = self.remove(client_addr) {
            entry.cleanup_all();
        }
    }
}
