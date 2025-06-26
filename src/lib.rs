// lib.rs or main module root
// -----------------------------------------------------------------------------
// Declares public and private submodules of the FTP server crate, organizing
// functionalities such as authentication, client management, command parsing,
// data channel handling, file transfers, and request processing.
//
// The `server` module is private and contains the core server implementation.
// The `start_server` function serves as the main entry point to initialize and
// run the FTP server instance.
// -----------------------------------------------------------------------------

/// Authentication-related logic: user validation, password checking, and error handling.
pub mod auth;

/// Manages data channel connections and registry for active FTP data transfers.
pub mod channel_registry;

/// Defines client state, authentication status, and connection information.
pub mod client;

/// Handles client connection lifecycle and processing of FTP commands.
pub mod client_handler;

/// FTP command definitions, parsing logic, and command result types.
pub mod command;

/// Manages accepting and handling FTP data connections for file transfers.
pub mod data_channel;

/// Implements file upload/download handlers with proper FTP status management.
pub mod file_transfer;

/// General command handlers mapping commands to business logic implementations.
pub mod handlers;

/// Core server implementation - private module managing listener, event loop, etc.
mod server;

/// Starts the FTP server by creating a new server instance and running it.
///
/// This function serves as the external entry point to launch the FTP server.
///
/// # Example
/// ```
/// ftp_server::start_server();
/// ```
pub fn start_server() {
    // Instantiate a new server object
    let server = server::Server::new();

    // Start the server event loop, listening for client connections
    server.start();
}
