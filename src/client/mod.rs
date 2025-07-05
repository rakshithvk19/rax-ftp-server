//! Client management system
//! 
//! Handles client connections, state management, and session lifecycle.

pub mod state;
pub mod handler;
pub mod session;
pub mod registry;

pub use state::Client;
pub use handler::handle_client;
