//! Module `command`
//!
//! Defines the core FTP command parsing logic and related data structures
//! used to represent commands, their status, associated data, and results.

/// Represents an FTP command parsed from the client input.
///
/// Each variant corresponds to a standard FTP command or custom extensions.
/// Commands that require arguments store them as `String` variants.
#[derive(Debug, PartialEq)]
pub enum Command {
    QUIT,
    LIST,
    LOGOUT,
    PWD,
    CWD(String),  // Change working directory
    USER(String), // Username for login
    PASS(String), // Password for login
    RETR(String), // Retrieve/download file
    STOR(String), // Store/upload file
    DEL(String),  // Delete file
    PORT(String), // Active mode data port specification
    PASV,         // Enter passive mode
    UNKNOWN,      // Unknown or unsupported command
    RAX,          // Custom command, e.g., server info or ping
}

/// Represents the outcome status of executing a command.
pub enum CommandStatus {
    Success,
    Failure(String),
    CloseConnection,
}

/// Additional data associated with a command result.
// #[derive(Debug)]
// pub enum CommandData {
//     /// Placeholder variant - currently unused
//     None,
// }

/// Struct encapsulating the full result of a command execution.
pub struct CommandResult {
    pub status: CommandStatus,
    pub message: Option<String>,
    // pub data: Option<CommandData>,
}

/// Parses a raw command string received from a client into the `Command` enum.
///
/// Validates required arguments and returns `UNKNOWN` if a known command is misused.
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
        "CWD" if !arg.is_empty() => Command::CWD(arg.to_string()),
        "USER" if !arg.is_empty() => Command::USER(arg.to_string()),
        "PASS" if !arg.is_empty() => Command::PASS(arg.to_string()),
        "RETR" if !arg.is_empty() => Command::RETR(arg.to_string()),
        "STOR" if !arg.is_empty() => Command::STOR(arg.to_string()),
        "DEL" if !arg.is_empty() => Command::DEL(arg.to_string()),
        "PORT" if !arg.is_empty() => Command::PORT(arg.to_string()),
        "PASV" => Command::PASV,
        "RAX" => Command::RAX,
        _ => Command::UNKNOWN,
    }
}
