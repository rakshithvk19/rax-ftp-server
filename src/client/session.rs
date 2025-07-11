//! Client session management
//!
//! Handles client session lifecycle and state transitions.

use crate::client::Client;

/// Manages client session lifecycle
pub struct ClientSession {
    client: Client,
}

impl ClientSession {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub fn get_client_mut(&mut self) -> &mut Client {
        &mut self.client
    }
}
