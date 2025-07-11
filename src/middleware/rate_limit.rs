//! Rate limiting middleware
//!
//! Provides rate limiting functionality.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Simple rate limiter
pub struct RateLimiter {
    requests: HashMap<String, Vec<Instant>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: HashMap::new(),
            max_requests,
            window,
        }
    }

    pub fn is_allowed(&mut self, client_id: &str) -> bool {
        let now = Instant::now();
        let entry = self
            .requests
            .entry(client_id.to_string())
            .or_insert_with(Vec::new);

        // Remove old requests
        entry.retain(|&time| now.duration_since(time) <= self.window);

        // Check if under limit
        if entry.len() < self.max_requests {
            entry.push(now);
            true
        } else {
            false
        }
    }
}
