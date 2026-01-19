use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::time;
use tracing::{error, info, warn};

use crate::api::PolymarketClient;
use crate::models::ActiveMarkets;

/// Worker that periodically scans Polymarket for active Dota 2 markets
pub struct MarketScannerWorker {
    client: PolymarketClient,
    active_markets: Arc<RwLock<ActiveMarkets>>,
    scan_interval: Duration,
}

impl MarketScannerWorker {
    /// Create a new market scanner worker
    pub fn new(
        client: PolymarketClient,
        active_markets: Arc<RwLock<ActiveMarkets>>,
        scan_interval_secs: u64,
    ) -> Self {
        Self {
            client,
            active_markets,
            scan_interval: Duration::from_secs(scan_interval_secs),
        }
    }

    /// Run the worker loop
    pub async fn run(&self) {
        info!(
            "Market scanner started (interval: {:?})",
            self.scan_interval
        );

        // Run initial scan immediately
        self.scan().await;

        // Then run on interval
        let mut interval = time::interval(self.scan_interval);
        interval.tick().await; // Skip first tick (already ran)

        loop {
            interval.tick().await;
            self.scan().await;
        }
    }

    /// Perform a single market scan
    async fn scan(&self) {
        info!("Scanning Polymarket for Dota 2 markets...");

        match self.client.fetch_dota2_markets().await {
            Ok(markets) => {
                let count = markets.len();

                // Update shared state
                let mut active = self.active_markets.write().await;
                active.clear();

                for market in markets {
                    info!(
                        "Found market: {} - {} vs {} (liquidity: ${:.2})",
                        market.condition_id, market.team_a, market.team_b, market.liquidity
                    );
                    active.insert(market.condition_id.clone(), market);
                }

                info!("Market scan complete: {} active markets", count);
            }
            Err(e) => {
                error!("Failed to scan markets: {}", e);
                warn!("Will retry on next interval");
            }
        }
    }
}
