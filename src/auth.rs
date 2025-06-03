use std::io::Write;
use std::net::TcpStream;

#[derive(Default)]
pub struct AuthState {
    is_user_valid: bool,
    is_logged_in: bool,
}

impl AuthState {
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

    pub fn is_logged_in(&self) -> bool {
        self.is_logged_in
    }

    pub fn logout(&mut self) {
        self.is_logged_in = false;
        self.is_user_valid = false;
    }

    // pub fn set_logged_in(&mut self, logged_in: bool) {
    //     self.is_logged_in = logged_in;
    // }

    // pub fn set_user_valid(&mut self, valid: bool) {
    //     self.is_user_valid = valid;
    // }

    // pub fn is_user_valid(&self) -> bool {
    //     self.is_user_valid
    // }
}
