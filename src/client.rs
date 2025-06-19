use std::net::{SocketAddr, TcpListener};

#[derive(Default)]
pub struct Client {
    is_user_valid: bool,
    is_logged_in: bool,
    is_data_channel_init: bool,
    data_listener: Option<TcpListener>,
    data_port: Option<u16>,
    data_socket: Option<SocketAddr>,
    username: Option<String>, 
}

impl Client {
    pub fn logout(&mut self) {
        self.is_user_valid = false;
        self.is_logged_in = false;
        self.username = None;
        self.data_listener = None;
        self.data_port = None;
        self.data_socket = None;
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

    pub fn data_listener(&self) -> &Option<TcpListener> {
        &self.data_listener
    }

    pub fn data_port(&self) -> Option<u16> {
        self.data_port
    }

    pub fn data_socket(&self) -> Option<SocketAddr> {
        self.data_socket
    }

    pub fn username(&self) -> Option<&String> {
        self.username.as_ref()
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

    pub fn set_data_listener(&mut self, listener: Option<TcpListener>) {
        self.data_listener = listener;
    }

    pub fn set_data_port(&mut self, port: Option<u16>) {
        self.data_port = port;
    }

    pub fn set_data_socket(&mut self, socket: Option<SocketAddr>) {
        self.data_socket = socket;
    }

    pub fn set_username(&mut self, username: Option<String>) {
        self.username = username;
    }

    pub fn take_data_listener(&mut self) -> Option<TcpListener> {
        self.data_listener.take()
    }
}