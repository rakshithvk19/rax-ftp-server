//! Configuration management for RAX FTP Server
//!
//! Separates startup configuration (requires restart) from runtime configuration
//! (can be updated via server terminal commands).

use config::{Config, Environment, File};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Complete server configuration with startup/runtime separation
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(flatten)]
    pub startup: StartupConfig,

    #[serde(flatten)]
    pub runtime: RuntimeConfig,
}

/// Configuration that requires server restart to take effect
/// These values are loaded once during server initialization
#[derive(Debug, Deserialize, Clone)]
pub struct StartupConfig {
    // ═══ NETWORK INFRASTRUCTURE (Environment Override Supported) ═══
    /// IP address to bind the FTP control connection (restart required)
    pub bind_address: String,

    /// Port for FTP control connection (restart required)
    pub control_port: u16,

    /// Port range for PASV data connections (restart required)
    pub data_port_min: u16,
    pub data_port_max: u16,

    /// Root directory for FTP operations (restart required)
    pub server_root: String,

    // ═══ INTERNAL BEHAVIOR (TOML Only) ═══
    /// Buffer size for file transfers (restart required)
    pub buffer_size: usize,

    /// Connection timeout for data channels (restart required)
    pub connection_timeout_secs: u64,

    /// Maximum retry attempts (restart required)
    pub max_retries: usize,

    /// Maximum FTP command length (restart required)
    pub max_command_length: usize,

    /// Security limits (restart required)
    pub max_directory_depth: usize,
    pub max_username_length: usize,
    pub min_client_port: u16,
}

/// Configuration that can be updated at runtime via terminal commands
/// These values can be changed while the server is running
#[derive(Debug, Deserialize, Clone)]
pub struct RuntimeConfig {
    /// Maximum concurrent clients (runtime updatable)
    /// Environment: RAX_FTP_MAX_CLIENTS
    pub max_clients: usize,

    /// Maximum file upload size in MB (runtime updatable)  
    /// Environment: RAX_FTP_MAX_FILE_SIZE_MB
    pub max_file_size_mb: u64,
}

/// Thread-safe runtime configuration wrapper
pub type SharedRuntimeConfig = Arc<RwLock<RuntimeConfig>>;

impl ServerConfig {
    /// Load configuration from config.toml with environment overrides
    pub fn load() -> Result<Self, config::ConfigError> {
        // Try production path first, then development path
        let config_paths = vec![
            "rax-ftp-server/config", // Docker production: /app/rax-ftp-server/config.toml
            "config",                // Local development: ./config.toml
        ];

        let mut last_error = None;

        for config_path in &config_paths {
            match Config::builder()
                .add_source(File::with_name(config_path))
                .add_source(Environment::with_prefix("RAX_FTP").separator("_"))
                .build()
            {
                Ok(settings) => {
                    let config: ServerConfig = settings.try_deserialize()?;
                    config.validate()?;
                    return Ok(config);
                }
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            }
        }

        // If both paths failed, panic with clear message
        panic!(
            "Failed to load config.toml from any location. Tried: {config_paths:?}. Last error: {last_error:?}"
        );
    }

    /// Split into startup (immutable) and runtime (mutable) parts
    pub fn split(self) -> (StartupConfig, SharedRuntimeConfig) {
        let runtime = Arc::new(RwLock::new(self.runtime));
        (self.startup, runtime)
    }

    /// Validation for all configuration values
    fn validate(&self) -> Result<(), config::ConfigError> {
        // Validate startup config
        if self.startup.control_port == 0 {
            return Err(config::ConfigError::Message(
                "Control port cannot be 0".into(),
            ));
        }

        if self.startup.data_port_min >= self.startup.data_port_max {
            return Err(config::ConfigError::Message(
                "data_port_min must be less than data_port_max".into(),
            ));
        }

        if self.startup.data_port_max - self.startup.data_port_min < 10 {
            return Err(config::ConfigError::Message(
                "Data port range too small (need at least 10 ports)".into(),
            ));
        }

        if self.startup.server_root.is_empty() {
            return Err(config::ConfigError::Message(
                "server_root cannot be empty".into(),
            ));
        }

        // Validate runtime config
        if self.runtime.max_clients == 0 {
            return Err(config::ConfigError::Message(
                "max_clients must be greater than 0".into(),
            ));
        }

        if self.runtime.max_file_size_mb == 0 {
            return Err(config::ConfigError::Message(
                "max_file_size_mb must be greater than 0".into(),
            ));
        }

        Ok(())
    }
}

impl StartupConfig {
    /// Get bind address and control port as socket address  
    pub fn control_socket(&self) -> String {
        format!("{}:{}", self.bind_address, self.control_port)
    }

    /// Get data port range for PASV mode
    pub fn data_port_range(&self) -> std::ops::Range<u16> {
        self.data_port_min..self.data_port_max
    }

    /// Get server root as PathBuf
    pub fn server_root_path(&self) -> PathBuf {
        PathBuf::from(&self.server_root)
    }

    /// Get server root as string (backward compatibility)
    pub fn server_root_str(&self) -> String {
        self.server_root.clone()
    }

    /// Get connection timeout as Duration
    pub fn connection_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.connection_timeout_secs)
    }
}

impl RuntimeConfig {
    /// Get maximum file size in bytes
    pub fn max_file_size_bytes(&self) -> u64 {
        self.max_file_size_mb * 1024 * 1024
    }
}
