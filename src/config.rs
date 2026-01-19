use std::env;

use anyhow::{Context, Result};

/// Application configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    /// Polymarket Gamma API URL
    pub polymarket_api_url: String,

    /// Interval in seconds for scanning Polymarket markets
    pub polymarket_scan_interval: u64,

    /// Interval in seconds for polling live match data
    pub live_match_poll_interval: u64,

    /// SQLite database path
    pub database_url: String,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Config {
            polymarket_api_url: env::var("POLYMARKET_API_URL")
                .unwrap_or_else(|_| "https://gamma-api.polymarket.com".to_string()),

            polymarket_scan_interval: env::var("POLYMARKET_SCAN_INTERVAL")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .context("POLYMARKET_SCAN_INTERVAL must be a valid number")?,

            live_match_poll_interval: env::var("LIVE_MATCH_POLL_INTERVAL")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .context("LIVE_MATCH_POLL_INTERVAL must be a valid number")?,

            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data/signals.db".to_string()),
        })
    }
}
