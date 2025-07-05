//! Logging utilities
//!
//! Provides logging setup and configuration.

use env_logger;

/// Setup logging for the server
pub fn setup_logging() {
    env_logger::init();
}
