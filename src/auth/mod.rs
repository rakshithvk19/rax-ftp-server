//! Authentication system
//!
//! Handles user authentication, credential validation, and session management.

pub mod credentials;
pub mod validator;

pub use validator::{AuthError, validate_password, validate_user};
