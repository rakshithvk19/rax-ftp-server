use std::net::TcpListener;

pub const DATA_PORT_RANGE: std::ops::Range<u16> = 2122..2222;

pub fn find_available_port() -> Option<u16> {
    for port in DATA_PORT_RANGE {
        if TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok() {
            return Some(port);
        }
    }
    None
}