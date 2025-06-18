use std::net::SocketAddr;

// Command enum to represent FTP commands
#[derive(Debug, PartialEq)]
pub enum Command {
    QUIT,
    LIST,
    LOGOUT,
    PWD,
    CWD(String),
    USER(String),
    PASS(String),
    RETR(String),
    STOR(String),
    PORT(String),
    PASV(),
    UNKNOWN(String),
}

#[derive(Debug, PartialEq)]
pub enum CommandResult {
    QUIT,
    CONTINUE,
    CONNECT(Option<SocketAddr>),
    STOR(String),
    RETR(String),
    LIST,
}

// Parse raw command string into Command enum
pub fn parse_command(raw: &str) -> Command {
    let trimmed = raw.trim();
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let cmd = parts.next().unwrap_or("").to_ascii_uppercase();
    let arg = parts.next().unwrap_or("").trim();

    match cmd.as_str() {
        "QUIT" | "Q" => Command::QUIT,
        "LIST" => Command::LIST,
        "LOGOUT" => Command::LOGOUT,
        "PWD" => Command::PWD,
        "CWD" => Command::CWD(arg.to_string()),
        "USER" => Command::USER(arg.to_string()),
        "PASS" => Command::PASS(arg.to_string()),
        "RETR" => Command::RETR(arg.to_string()),
        "STOR" => Command::STOR(arg.to_string()),
        "PORT" => Command::PORT(arg.to_string()),
        "PASV" => Command::PASV(),
        _ => Command::UNKNOWN(trimmed.to_string()),
    }
}
