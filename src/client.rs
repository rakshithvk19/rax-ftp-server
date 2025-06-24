//client.rs

use std::net::SocketAddr;

#[derive(Default)]
pub struct Client {
    username: Option<String>,
    client_addr: Option<SocketAddr>,
    is_user_valid: bool,
    is_logged_in: bool,
    is_data_channel_init: bool,
}

impl Client {
    pub fn logout(&mut self) {
        self.username = None;
        self.client_addr = None;
        self.is_user_valid = false;
        self.is_logged_in = false;
        self.is_data_channel_init = false;
    }

    // Getters
    pub fn is_user_valid(&self) -> bool {
        self.is_user_valid
    }

    pub fn is_logged_in(&self) -> bool {
        self.is_logged_in
    }

    pub fn is_data_channel_init(&self) -> bool {
        self.is_data_channel_init
    }

    pub fn username(&self) -> Option<&String> {
        self.username.as_ref()
    }

    pub fn client_addr(&self) -> Option<&SocketAddr> {
        self.client_addr.as_ref()
    }

    // Setters
    pub fn set_user_valid(&mut self, valid: bool) {
        self.is_user_valid = valid;
    }

    pub fn set_logged_in(&mut self, logged_in: bool) {
        self.is_logged_in = logged_in;
    }

    pub fn set_data_channel_init(&mut self, init: bool) {
        self.is_data_channel_init = init;
    }

    pub fn set_username(&mut self, username: Option<String>) {
        self.username = username;
    }

    pub fn set_client_addr(&mut self, addr: Option<SocketAddr>) {
        self.client_addr = addr;
    }
}
