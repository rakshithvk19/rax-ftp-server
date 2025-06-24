// channel_registry.rs
// Manages a registry of data channels for FTP connections.

use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};

#[derive(Default)]
pub struct ChannelEntry {
    data_socket: Option<SocketAddr>,
    data_stream: Option<TcpStream>,
    listener: Option<TcpListener>,
}

impl ChannelEntry {
    // Getters
    pub fn data_socket(&self) -> Option<&SocketAddr> {
        self.data_socket.as_ref()
    }

    pub fn data_stream(&self) -> Option<&TcpStream> {
        self.data_stream.as_ref()
    }

    pub fn listener(&self) -> Option<&TcpListener> {
        self.listener.as_ref()
    }

    // Mutable getters
    pub fn data_socket_mut(&mut self) -> Option<&mut SocketAddr> {
        self.data_socket.as_mut()
    }

    pub fn data_stream_mut(&mut self) -> Option<&mut TcpStream> {
        self.data_stream.as_mut()
    }

    pub fn listener_mut(&mut self) -> Option<&mut TcpListener> {
        self.listener.as_mut()
    }

    // Setters
    pub fn set_data_socket(&mut self, socket: Option<SocketAddr>) {
        self.data_socket = socket;
    }

    pub fn set_data_stream(&mut self, stream: Option<TcpStream>) {
        self.data_stream = stream;
    }

    pub fn set_listener(&mut self, listener: Option<TcpListener>) {
        self.listener = listener;
    }

    // Take ownership (remove from struct and return)
    pub fn take_data_socket(&mut self) -> Option<SocketAddr> {
        self.data_socket.take()
    }

    pub fn take_data_stream(&mut self) -> Option<TcpStream> {
        self.data_stream.take()
    }

    pub fn take_listener(&mut self) -> Option<TcpListener> {
        self.listener.take()
    }
}

#[derive(Default)]
pub struct ChannelRegistry {
    registry: HashMap<SocketAddr, ChannelEntry>,
}

impl ChannelRegistry {
    /// Port range used for PASV mode data channels
    pub const DATA_PORT_RANGE: std::ops::Range<u16> = 2122..2222;

    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    /// Insert a data channel entry associated with a socket address
    pub fn insert(&mut self, addr: SocketAddr, entry: ChannelEntry) {
        self.registry.insert(addr, entry);
    }

    /// Remove and return the data channel entry for the given address
    pub fn remove(&mut self, addr: &SocketAddr) -> Option<ChannelEntry> {
        self.registry.remove(addr)
    }

    /// Get a reference to a data channel entry without removing it
    pub fn get(&self, addr: &SocketAddr) -> Option<&ChannelEntry> {
        self.registry.get(addr)
    }

    /// Get a mutable reference to a data channel entry
    pub fn get_mut(&mut self, addr: &SocketAddr) -> Option<&mut ChannelEntry> {
        self.registry.get_mut(addr)
    }

    /// Check if an entry exists for a given address
    pub fn contains(&self, addr: &SocketAddr) -> bool {
        self.registry.contains_key(addr)
    }

    /// List all registered addresses
    pub fn list_addresses(&self) -> Vec<SocketAddr> {
        self.registry.keys().cloned().collect()
    }

    /// Finds the next available socket address within the defined range that is not in use
    pub fn next_available_socket(&self) -> Option<SocketAddr> {
        for port in Self::DATA_PORT_RANGE {
            let data_socket: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

            // Check if any existing ChannelEntry has this socket as its data_socket
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

    /// Checks if the given socket address is already used by any other client's data_socket
    pub fn is_socket_taken(&self, addr: &SocketAddr) -> bool {
        self.registry
            .values()
            .any(|entry| entry.data_socket.as_ref() == Some(addr))
    }
}
