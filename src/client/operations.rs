//! Client operations
//!
//! Handles client session management operations.

use crate::error::ClientError;
use crate::client::results::{LogoutResult, QuitResult};
use crate::client::Client;

/// Handles client logout
pub fn process_logout(client: &mut Client) -> Result<LogoutResult, ClientError> {
    let was_logged_in = client.is_logged_in();
    
    if was_logged_in {
        client.logout();
        Ok(LogoutResult { was_logged_in: true })
    } else {
        Err(ClientError::InvalidState("User not logged in".into()))
    }
}

/// Handles client quit/disconnect
pub fn process_quit(client: &mut Client) -> Result<QuitResult, ClientError> {
    let client_addr = client.client_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    
    client.logout();
    
    Ok(QuitResult { client_addr })
}
