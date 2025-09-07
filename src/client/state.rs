//! Module `client`
//!
//! Defines the `Client` struct and associated methods to manage FTP client state,
//! including authentication status, connection address, and data channel initialization.

use crate::config::StartupConfig;
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
        if self.is_logged_in {
            log::info!(
                "Logging out client {} (user: {})",
                self.client_addr
                    .map(|addr| addr.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                self.username.as_ref().unwrap_or(&"unknown".to_string())
            );
        }

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
    pub fn set_logged_in(&mut self, logged_in: bool) {
        if logged_in && !self.is_logged_in {
            log::info!(
                "Client {} successfully logged in as user {}",
                self.client_addr
                    .map(|addr| addr.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                self.username.as_ref().unwrap_or(&"unknown".to_string())
            );
        }
        self.is_logged_in = logged_in;
    }

    /// Sets the initialization state of the data channel.
    ///
    /// Indicates whether the client has established a data connection.
    pub fn set_data_channel_init(&mut self, init: bool) {
        self.is_data_channel_init = init;
    }

    /// Sets the username of the client with validation
    pub fn set_username(
        &mut self,
        username: Option<String>,
        config: &StartupConfig,
    ) -> Result<(), String> {
        if let Some(ref new_username) = username {
            // Validate username
            if new_username.is_empty() {
                return Err("Username cannot be empty".to_string());
            }
            if new_username.len() > config.max_username_length {
                return Err(format!(
                    "Username too long (max {} characters)",
                    config.max_username_length
                ));
            }
            if new_username.contains('\0')
                || new_username.contains('\n')
                || new_username.contains('\r')
            {
                return Err("Username contains invalid characters".to_string());
            }

            log::info!(
                "Client {} set username to: {}",
                self.client_addr
                    .map(|addr| addr.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                new_username
            );
        }
        self.username = username;
        Ok(())
    }

    /// Sets the client's socket address.
    pub fn set_client_addr(&mut self, addr: Option<SocketAddr>) {
        self.client_addr = addr;
    }

    /// Sets the current virtual path of the client.
    /// Sets the current virtual path of the client with validation
    pub fn set_current_virtual_path(&mut self, path: String) -> Result<(), String> {
        // Validate virtual path
        if path.is_empty() {
            return Err("Virtual path cannot be empty".to_string());
        }
        if !path.starts_with('/') {
            return Err("Virtual path must start with /".to_string());
        }
        if path.contains('\0') {
            return Err("Virtual path contains null characters".to_string());
        }
        if path.contains("..") {
            return Err("Virtual path cannot contain directory traversal".to_string());
        }

        log::info!(
            "Client {} changed virtual path to: {}",
            self.client_addr
                .map(|addr| addr.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            path
        );

        self.current_virtual_path = path;
        Ok(())
    }
}
