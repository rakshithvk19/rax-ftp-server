//! Authentication system
//!
//! Handles user authentication and credential validation.

mod credentials;
pub mod validator;

pub use validator::{validate_password, validate_user};
