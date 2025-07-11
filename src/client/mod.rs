//! Client management system
//!
//! Handles client connections, state management, and session lifecycle.

pub mod handler;
mod operations;
pub mod registry;
mod results;
pub mod session;
pub mod state;

pub use handler::handle_client;
pub use operations::{process_logout, process_quit};
pub use results::{LogoutResult, QuitResult};
pub use state::Client;
