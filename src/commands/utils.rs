use crate::auth::AuthState;
use std::net::TcpStream;

pub fn require_auth(auth_state: &mut AuthState, stream: &mut TcpStream) -> bool {
    if !auth_state.is_logged_in() {
        let _ = stream.write_all(b"530 Not logged in\r\n");
        false
    } else {
        true
    }
}
