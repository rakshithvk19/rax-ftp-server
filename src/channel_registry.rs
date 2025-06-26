//! Module `channel_registry`
//!
//! Provides a centralized registry to manage FTP data channels per client,
//! including active data sockets, TCP streams, and passive-mode listeners.
//! Facilitates allocation and lifecycle management of data connections used
//! for file transfers (e.g., STOR, RETR, LIST).

use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};

/// Represents the state of a single FTP data channel associated with a client.
/// Contains optional references to the client's data socket address,
/// the active data stream, and any passive mode listener.
#[derive(Default)]
pub struct ChannelEntry {
    data_socket: Option<SocketAddr>, // IP:Port the client uses for active data connection
    data_stream: Option<TcpStream>,  // Established TCP stream for the data transfer
    listener: Option<TcpListener>,   // Listener socket for passive mode connections
}

impl ChannelEntry {
    // --- Accessors ---

    /// Returns a reference to the data socket address if present.
    pub fn data_socket(&self) -> Option<&SocketAddr> {
        self.data_socket.as_ref()
    }

    /// Returns a reference to the TCP stream if present.
    pub fn data_stream(&self) -> Option<&TcpStream> {
        self.data_stream.as_ref()
    }

    /// Returns a reference to the passive mode TCP listener if present.
    pub fn listener(&self) -> Option<&TcpListener> {
        self.listener.as_ref()
    }

    // --- Mutable Accessors ---

    /// Returns a mutable reference to the data socket address if present.
    pub fn data_socket_mut(&mut self) -> Option<&mut SocketAddr> {
        self.data_socket.as_mut()
    }

    /// Returns a mutable reference to the TCP stream if present.
    pub fn data_stream_mut(&mut self) -> Option<&mut TcpStream> {
        self.data_stream.as_mut()
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

    // --- Take Ownership Methods ---

    /// Takes ownership of the data socket out of the entry, leaving None behind.
    pub fn take_data_socket(&mut self) -> Option<SocketAddr> {
        self.data_socket.take()
    }

    /// Takes ownership of the data stream out of the entry, leaving None behind.
    pub fn take_data_stream(&mut self) -> Option<TcpStream> {
        self.data_stream.take()
    }

    /// Takes ownership of the listener out of the entry, leaving None behind.
    pub fn take_listener(&mut self) -> Option<TcpListener> {
        self.listener.take()
    }
}

/// Registry that maps client socket addresses to their corresponding FTP data channels.
/// Manages allocation and bookkeeping of active data connections.
#[derive(Default)]
pub struct ChannelRegistry {
    registry: HashMap<SocketAddr, ChannelEntry>,
}

impl ChannelRegistry {
    /// Port range used for PASV (passive) mode data channel listeners.
    /// The server listens on these ports to accept incoming client data connections.
    pub const DATA_PORT_RANGE: std::ops::Range<u16> = 2122..2222;

    /// Creates a new, empty ChannelRegistry.
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    /// Inserts or replaces the data channel entry associated with the given client address.
    ///
    /// # Arguments
    /// * `addr` - The client socket address (control connection) as key.
    /// * `entry` - The ChannelEntry containing data channel state for this client.
    pub fn insert(&mut self, addr: SocketAddr, entry: ChannelEntry) {
        self.registry.insert(addr, entry);
    }

    /// Removes and returns the data channel entry for a given client address, if any.
    ///
    /// # Arguments
    /// * `addr` - Client socket address key.
    ///
    /// # Returns
    /// Optionally returns the removed ChannelEntry.
    pub fn remove(&mut self, addr: &SocketAddr) -> Option<ChannelEntry> {
        self.registry.remove(addr)
    }

    /// Returns an immutable reference to the data channel entry for a client address, if present.
    pub fn get(&self, addr: &SocketAddr) -> Option<&ChannelEntry> {
        self.registry.get(addr)
    }

    /// Returns a mutable reference to the data channel entry for a client address, if present.
    pub fn get_mut(&mut self, addr: &SocketAddr) -> Option<&mut ChannelEntry> {
        self.registry.get_mut(addr)
    }

    /// Checks whether a data channel entry exists for the given client address.
    pub fn contains(&self, addr: &SocketAddr) -> bool {
        self.registry.contains_key(addr)
    }

    /// Returns a list of all client addresses currently registered.
    pub fn list_addresses(&self) -> Vec<SocketAddr> {
        self.registry.keys().cloned().collect()
    }

    /// Attempts to find the next available socket address in the configured PASV port range
    /// that is not currently assigned to any client's data socket.
    ///
    /// # Returns
    /// An available `SocketAddr` on localhost with an unused port, or `None` if none are free.
    pub fn next_available_socket(&self) -> Option<SocketAddr> {
        for port in Self::DATA_PORT_RANGE {
            // Construct localhost address with candidate port
            let data_socket: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

            // Check if this port is already in use by any channel entry's data_socket
            let in_use = self
                .registry
                .values()
                .any(|entry| entry.data_socket == Some(data_socket));

            if !in_use {
                return Some(data_socket);
            }
        }
        None
    }

    /// Checks if the given socket address is already assigned as a data socket for any client.
    ///
    /// # Arguments
    /// * `addr` - Socket address to check.
    ///
    /// # Returns
    /// `true` if the address is in use, `false` otherwise.
    pub fn is_socket_taken(&self, addr: &SocketAddr) -> bool {
        self.registry
            .values()
            .any(|entry| entry.data_socket.as_ref() == Some(addr))
    }
}
