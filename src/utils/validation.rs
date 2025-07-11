//! Input validation utilities
//!
//! Provides input validation and sanitization functions.

/// Validate that input is not empty and doesn't contain dangerous characters
pub fn is_valid_input(input: &str) -> bool {
    !input.trim().is_empty()
        && input.len() <= 512
        && !input.contains('\0')
        && !input.contains('\r')
        && !input.contains('\n')
}

/// Sanitize user input
pub fn sanitize_input(input: &str) -> String {
    input.trim().to_string()
}
