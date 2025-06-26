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
    PASV(),       // Enter passive mode
    UNKNOWN,      // Unknown or unsupported command
    RAX,          // Custom command, e.g., server info or ping
}

/// Represents the outcome status of executing a command.
///
/// Variants:
/// - `Success`: Command succeeded.
/// - `Failure(String)`: Command failed, with error message.
/// - `CloseConnection`: Server should close client connection.
pub enum CommandStatus {
    Success,
    Failure(String),
    CloseConnection,
}

/// Additional data associated with a command result.
///
/// For example, a directory listing returned by the LIST command.
pub enum CommandData {
    DirectoryListing(Vec<String>),
}

/// Struct encapsulating the full result of a command execution.
///
/// - `status`: Outcome of the command (success/failure/close).
/// - `message`: Optional text message to send to client (e.g., FTP response codes).
/// - `data`: Optional data payload (e.g., directory listing).
pub struct CommandResult {
    pub status: CommandStatus,
    pub message: Option<String>,
    pub data: Option<CommandData>,
}

/// Parses a raw command string received from a client into the `Command` enum.
///
/// This function splits the input on the first whitespace, identifies
/// the command keyword (case-insensitive), and parses any argument string.
///
/// # Arguments
///
/// * `raw` - Raw command string from client (e.g., `"USER alice"`)
///
/// # Returns
///
/// Parsed `Command` enum variant with associated arguments as applicable.
/// Returns `Command::UNKNOWN` for unrecognized commands.
///
/// # Examples
///
/// ```
/// let cmd = parse_command("USER alice");
/// assert_eq!(cmd, Command::USER("alice".to_string()));
/// ```
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
        "DEL" => Command::DEL(arg.to_string()),
        "PORT" => Command::PORT(arg.to_string()),
        "PASV" => Command::PASV(),
        "RAX" => Command::RAX,
        _ => Command::UNKNOWN,
    }
}
