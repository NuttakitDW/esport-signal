mod api;
mod config;
mod db;
mod matching;
mod models;
mod workers;

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::api::{PolymarketClient, StratzClient};
use crate::config::Config;
use crate::db::SignalStore;
use crate::matching::TeamResolver;
use crate::models::{ActiveMarkets, LiveMatchCache};
use crate::workers::{LiveFetcherWorker, MarketScannerWorker, SignalProcessorWorker};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "esport_signal=info,warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting esport-signal");

    // Load configuration
    let config = Config::from_env()?;
    info!("Configuration loaded");

    // Initialize database
    let signal_store = Arc::new(SignalStore::new(&config.database_url).await?);
    info!("Database initialized");

    // Load team aliases
    let team_resolver = load_team_resolver()?;
    let team_resolver = Arc::new(team_resolver);
    info!("Team resolver initialized");

    // Initialize API clients
    let polymarket_client = PolymarketClient::new(&config.polymarket_api_url);
    let stratz_client = StratzClient::new(&config.stratz_api_token);
    info!("API clients initialized");

    // Shared state
    let active_markets: Arc<RwLock<ActiveMarkets>> = Arc::new(RwLock::new(Default::default()));
    let match_cache: Arc<RwLock<LiveMatchCache>> = Arc::new(RwLock::new(Default::default()));

    // Channel for match updates
    let (update_tx, update_rx) = mpsc::channel(100);

    // Create workers
    let market_scanner = MarketScannerWorker::new(
        polymarket_client,
        Arc::clone(&active_markets),
        config.polymarket_scan_interval,
    );

    let live_fetcher = LiveFetcherWorker::new(
        stratz_client,
        Arc::clone(&active_markets),
        Arc::clone(&match_cache),
        Arc::clone(&team_resolver),
        update_tx,
        config.live_match_poll_interval,
    );

    let signal_processor = SignalProcessorWorker::new(
        Arc::clone(&active_markets),
        Arc::clone(&signal_store),
        update_rx,
    );

    info!("Workers created, starting...");

    // Spawn workers
    let scanner_handle = tokio::spawn(async move {
        market_scanner.run().await;
    });

    let fetcher_handle = tokio::spawn(async move {
        live_fetcher.run().await;
    });

    let processor_handle = tokio::spawn(async move {
        signal_processor.run().await;
    });

    info!("All workers started");

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
        result = scanner_handle => {
            error!("Market scanner exited unexpectedly: {:?}", result);
        }
        result = fetcher_handle => {
            error!("Live fetcher exited unexpectedly: {:?}", result);
        }
        result = processor_handle => {
            error!("Signal processor exited unexpectedly: {:?}", result);
        }
    }

    info!("Shutting down esport-signal");
    Ok(())
}

/// Load team resolver from JSON file or create default
fn load_team_resolver() -> Result<TeamResolver> {
    let aliases_path = Path::new("data/team_aliases.json");

    if aliases_path.exists() {
        TeamResolver::load_from_file(aliases_path)
    } else {
        info!("No team aliases file found, using default resolver");
        Ok(TeamResolver::new())
    }
}
