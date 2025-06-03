// Command enum to represent FTP commands
#[derive(Debug, PartialEq)]
pub enum Command {
    Quit,
    List,
    Logout,
    User(String),
    Pass(String),
    Retr(String),
    Stor(String),
    Unknown(String),
}

#[derive(Debug, PartialEq)]
pub enum CommandResult {
    Quit,
    Continue,
    Stor,
}

// Parse raw command string into Command enum
pub fn parse_command(raw: &str) -> Command {
    let trimmed = raw.trim();
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let cmd = parts.next().unwrap_or("").to_ascii_uppercase();
    let arg = parts.next().unwrap_or("").trim();

    match cmd.as_str() {
        "QUIT" | "Q" => Command::Quit,
        "LIST" => Command::List,
        "LOGOUT" => Command::Logout,
        "USER" => Command::User(arg.to_string()),
        "PASS" => Command::Pass(arg.to_string()),
        "RETR" => Command::Retr(arg.to_string()),
        "STOR" => Command::Stor(arg.to_string()),
        _ => Command::Unknown(trimmed.to_string()),
    }
}
