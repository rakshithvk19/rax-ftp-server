//client.rs

use std::io::Write;
use std::net::{TcpListener, TcpStream};

#[derive(Default)]
pub struct Client {
    is_user_valid: bool,
    is_logged_in: bool,
    is_data_channel_init: bool,
    data_listener: Option<TcpListener>,
    data_port: Option<u16>,
}

impl Client {
    pub fn handle_user(&mut self, username: &str, stream: &mut TcpStream) {
        //TODO: Replace with actual user validation logic
        if username == "user" {
            self.is_user_valid = true;
            self.is_logged_in = false;

            let _ = stream.write_all(b"331 Password required\r\n");
        } else {
            self.is_user_valid = false;
            self.is_logged_in = false;

            let _ = stream.write_all(b"530 Invalid username\r\n");
        }
    }

    pub fn handle_pass(&mut self, password: &str, stream: &mut TcpStream) {
        if self.is_user_valid {
            //TODO: Replace with actual password validation logic
            if password == "pass" {
                self.is_logged_in = true;
                let _ = stream.write_all(b"230 Login successful\r\n");
            } else {
                self.is_logged_in = false;
                let _ = stream.write_all(b"530 Invalid password\r\n");
            }
        } else {
            let _ = stream.write_all(b"530 Please enter the username first\r\n");
        }
    }

    pub fn logout(&mut self) {
        self.is_logged_in = false;
        self.is_user_valid = false;
        self.data_listener = None;
        self.data_port = None;
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

    // Mutable getter for data_listener to allow taking ownership
    pub fn take_data_listener(&mut self) -> Option<TcpListener> {
        self.data_listener.take()
    }
}
