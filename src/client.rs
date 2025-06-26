//! Module `client`
//!
//! Defines the `Client` struct and associated methods to manage FTP client state,
//! including authentication status, connection address, and data channel initialization.

use std::net::SocketAddr;

/// Represents the state of a connected FTP client.
///
/// Tracks authentication status, client address, and
/// whether the data channel for file transfers has been initialized.
#[derive(Default)]
pub struct Client {
    username: Option<String>,
    client_addr: Option<SocketAddr>,
    is_user_valid: bool,
    is_logged_in: bool,
    is_data_channel_init: bool,
}

impl Client {
    /// Resets the client state, logging out and clearing all stored data.
    ///
    /// This includes username, client address, authentication flags,
    /// and data channel initialization status.
    pub fn logout(&mut self) {
        self.username = None;
        self.client_addr = None;
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
}
