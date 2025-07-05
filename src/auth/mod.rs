//! Authentication system
//! 
//! Handles user authentication, credential validation, and session management.

pub mod validator;
pub mod credentials;

pub use validator::{validate_user, validate_password, AuthError};
