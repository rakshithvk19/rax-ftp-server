//! Authentication system
//!
//! Handles user authentication and credential validation.

mod credentials;
mod results;
pub mod validator;

pub use results::{PasswordValidationResult, UserValidationResult};
pub use validator::{validate_password, validate_user};
