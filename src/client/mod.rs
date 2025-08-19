//! Client management system
//!
//! Handles client connections, state management, and session lifecycle.

pub mod handler;
pub mod session;
pub mod state;

pub use handler::handle_client;
pub use state::Client;
