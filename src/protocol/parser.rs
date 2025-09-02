//! FTP Command parsing
//!
//! Handles parsing of FTP commands from client input.

use crate::protocol::Command;

/// Parse a command string into a Command enum
/// This is the main parsing function exported from commands.rs
pub use crate::protocol::commands::parse_command;