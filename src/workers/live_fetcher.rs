use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, RwLock};
use tokio::time;
use tracing::{debug, error, info, warn};

use crate::api::LiveDataClient;
use crate::matching::TeamResolver;
use crate::models::{ActiveMarkets, LiveMatchCache, MatchUpdate};

/// Worker that fetches live match data for active markets
pub struct LiveFetcherWorker {
    client: LiveDataClient,
    active_markets: Arc<RwLock<ActiveMarkets>>,
    match_cache: Arc<RwLock<LiveMatchCache>>,
    team_resolver: Arc<TeamResolver>,
    update_tx: mpsc::Sender<MatchUpdate>,
    poll_interval: Duration,
}

impl LiveFetcherWorker {
    /// Create a new live fetcher worker
    pub fn new(
        client: LiveDataClient,
        active_markets: Arc<RwLock<ActiveMarkets>>,
        match_cache: Arc<RwLock<LiveMatchCache>>,
        team_resolver: Arc<TeamResolver>,
        update_tx: mpsc::Sender<MatchUpdate>,
        poll_interval_secs: u64,
    ) -> Self {
        Self {
            client,
            active_markets,
            match_cache,
            team_resolver,
            update_tx,
            poll_interval: Duration::from_secs(poll_interval_secs),
        }
    }

    /// Run the worker loop
    pub async fn run(&self) {
        info!("Live fetcher started (interval: {:?})", self.poll_interval);

        let mut interval = time::interval(self.poll_interval);

        loop {
            interval.tick().await;
            self.fetch().await;
        }
    }

    /// Perform a single fetch cycle
    async fn fetch(&self) {
        // Check if we have any active markets
        let markets = self.active_markets.read().await;
        if markets.is_empty() {
            debug!("No active markets, skipping live data fetch");
            return;
        }

        let market_count = markets.len();
        drop(markets); // Release lock before API call

        debug!("Fetching live matches for {} active markets", market_count);

        // Fetch all live matches
        let live_matches = match self.client.fetch_live_matches().await {
            Ok(matches) => matches,
            Err(e) => {
                error!("Failed to fetch live matches: {}", e);
                return;
            }
        };

        if live_matches.is_empty() {
            debug!("No live matches found");
            return;
        }

        debug!("Found {} live matches", live_matches.len());

        // Match markets to live games
        let markets = self.active_markets.read().await;
        let mut cache = self.match_cache.write().await;

        for market in markets.values() {
            if let Some(match_result) =
                self.team_resolver.match_market_to_live(market, &live_matches)
            {
                let match_id = match_result.match_state.match_id;

                // Get previous state for comparison
                let previous_state = cache.get(&match_id).cloned();

                // Update cache
                cache.insert(match_id, match_result.match_state.clone());

                // Send update to signal processor
                let update = MatchUpdate {
                    market_condition_id: market.condition_id.clone(),
                    state: match_result.match_state,
                    previous_state,
                };

                if let Err(e) = self.update_tx.send(update).await {
                    warn!("Failed to send match update: {}", e);
                }
            }
        }
    }
}
