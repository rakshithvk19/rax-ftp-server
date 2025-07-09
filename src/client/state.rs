//! Module `client`
//!
//! Defines the `Client` struct and associated methods to manage FTP client state,
//! including authentication status, connection address, and data channel initialization.

use std::net::SocketAddr;

/// Represents the state of a connected FTP client.
///
/// Tracks authentication status, client address, virtual directory path,
/// and whether the data channel for file transfers has been initialized.
pub struct Client {
    username: Option<String>,
    client_addr: Option<SocketAddr>,
    current_virtual_path: String,
    is_user_valid: bool,
    is_logged_in: bool,
    is_data_channel_init: bool,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            username: None,
            client_addr: None,
            current_virtual_path: "/".to_string(),
            is_user_valid: false,
            is_logged_in: false,
            is_data_channel_init: false,
        }
    }
}

impl Client {
    /// Resets the client state, logging out and clearing all stored data.
    ///
    /// This includes username, client address, authentication flags,
    /// virtual path, and data channel initialization status.
    pub fn logout(&mut self) {
        self.username = None;
        self.client_addr = None;
        self.current_virtual_path = "/".to_string();
        self.is_user_valid = false;
        self.is_logged_in = false;
        self.is_data_channel_init = false;
    }

    // --------------------
    // Getter methods
    // --------------------

    /// Returns whether the username provided by the client is valid.
    ///
    /// This indicates if the USER command was accepted.
    pub fn is_user_valid(&self) -> bool {
        self.is_user_valid
    }

    /// Returns whether the client has successfully logged in (passed authentication).
    pub fn is_logged_in(&self) -> bool {
        self.is_logged_in
    }

    /// Returns whether the data channel for file transfers has been initialized.
    pub fn is_data_channel_init(&self) -> bool {
        self.is_data_channel_init
    }

    /// Returns the username of the client if set.
    pub fn username(&self) -> Option<&String> {
        self.username.as_ref()
    }

    /// Returns the client's socket address if known.
    pub fn client_addr(&self) -> Option<&SocketAddr> {
        self.client_addr.as_ref()
    }

    /// Returns the current virtual path of the client.
    pub fn current_virtual_path(&self) -> &str {
        &self.current_virtual_path
    }

    // --------------------
    // Setter methods
    // --------------------

    /// Sets the validity state of the username.
    ///
    /// Typically set after USER command validation.
    pub fn set_user_valid(&mut self, valid: bool) {
        self.is_user_valid = valid;
    }

    /// Sets the login state of the client.
    ///
    /// Typically set after successful PASS command validation.
    pub fn set_logged_in(&mut self, logged_in: bool) {
        self.is_logged_in = logged_in;
    }

    /// Sets the initialization state of the data channel.
    ///
    /// Indicates whether the client has established a data connection.
    pub fn set_data_channel_init(&mut self, init: bool) {
        self.is_data_channel_init = init;
    }

    /// Sets the username of the client.
    pub fn set_username(&mut self, username: Option<String>) {
        self.username = username;
    }

    /// Sets the client's socket address.
    pub fn set_client_addr(&mut self, addr: Option<SocketAddr>) {
        self.client_addr = addr;
    }

    /// Sets the current virtual path of the client.
    pub fn set_current_virtual_path(&mut self, path: String) {
        self.current_virtual_path = path;
    }
}
