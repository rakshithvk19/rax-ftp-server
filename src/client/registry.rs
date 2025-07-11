//! Client registry
//!
//! Manages registered clients and their tracking.

use crate::client::Client;
use std::collections::HashMap;
use std::net::SocketAddr;

/// Registry for tracking active clients
pub struct ClientRegistry {
    clients: HashMap<SocketAddr, Client>,
}

impl ClientRegistry {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub fn insert(&mut self, addr: SocketAddr, client: Client) {
        self.clients.insert(addr, client);
    }

    pub fn remove(&mut self, addr: &SocketAddr) -> Option<Client> {
        self.clients.remove(addr)
    }

    pub fn get(&self, addr: &SocketAddr) -> Option<&Client> {
        self.clients.get(addr)
    }

    pub fn get_mut(&mut self, addr: &SocketAddr) -> Option<&mut Client> {
        self.clients.get_mut(addr)
    }

    pub fn len(&self) -> usize {
        self.clients.len()
    }
}

impl Default for ClientRegistry {
    fn default() -> Self {
        Self::new()
    }
}
