//! Environment/config ports and data structures.
//!
//! Key rule: **do not rely on global `std::env` mutation** inside the application layer.
//! We load config into a `StoreConfig` value and pass it explicitly (supports future `diff`).

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    pub name: String,         // "default" or "<name>"
    pub display_name: String, // "Default" or capitalized "<name>"
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

impl LogFormat {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "json" => LogFormat::Json,
            _ => LogFormat::Pretty,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreConfig {
    pub env_name: String,
    pub shop_domain: String,
    pub access_token: String,
    pub log_format: LogFormat,
    /// Disable the best-effort update check.
    pub disable_update_check: bool,
    /// How often to check for updates on startup (in days).
    pub update_check_days: u64,
}

/// Port: environment discovery + loading.
///
/// Implementations live in `mds_infra` (filesystem + dotenv parsing).
pub trait EnvironmentService {
    fn detect(&self) -> Result<Vec<Environment>, crate::AppError>;
    fn load_store_config(&self, env: &Environment) -> Result<StoreConfig, crate::AppError>;
}

