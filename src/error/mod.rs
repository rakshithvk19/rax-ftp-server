//! Error management system
//!
//! Provides error types and handlers for the FTP server.

pub mod handlers;
pub mod types;

pub use types::{
    AuthError, NavigateError, StorageError, TransferError,
};
