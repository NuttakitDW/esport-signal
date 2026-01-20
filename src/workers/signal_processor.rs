use std::sync::Arc;

use chrono::Utc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

use crate::db::SignalStore;
use crate::models::{ActiveMarkets, MatchUpdate, Signal};

/// Worker that processes match updates and stores snapshots
pub struct SignalProcessorWorker {
    active_markets: Arc<RwLock<ActiveMarkets>>,
    signal_store: Arc<SignalStore>,
    update_rx: mpsc::Receiver<MatchUpdate>,
}

impl SignalProcessorWorker {
    /// Create a new signal processor worker
    pub fn new(
        active_markets: Arc<RwLock<ActiveMarkets>>,
        signal_store: Arc<SignalStore>,
        update_rx: mpsc::Receiver<MatchUpdate>,
    ) -> Self {
        Self {
            active_markets,
            signal_store,
            update_rx,
        }
    }

    /// Run the worker loop
    pub async fn run(mut self) {
        info!("Signal processor started");

        while let Some(update) = self.update_rx.recv().await {
            self.process_update(update).await;
        }

        warn!("Signal processor channel closed");
    }

    /// Process a match update and store snapshot
    async fn process_update(&self, update: MatchUpdate) {
        let markets = self.active_markets.read().await;

        let market = match markets.get(&update.market_condition_id) {
            Some(m) => m,
            None => {
                warn!(
                    "Market {} not found in active markets",
                    update.market_condition_id
                );
                return;
            }
        };

        // Create signal (match snapshot)
        let signal = Signal {
            id: None,
            market_condition_id: update.market_condition_id.clone(),
            match_id: update.state.match_id,
            market_team_a_odds: market.team_a_odds,
            match_snapshot: serde_json::to_string(&update.state).unwrap_or_default(),
            created_at: Utc::now(),
        };

        // Log
        info!(
            "Snapshot | Match {} | {} vs {} | Score: {}-{} | Gold: {}k | Market: {:.1}%",
            signal.match_id,
            update.state.radiant.name,
            update.state.dire.name,
            update.state.radiant.kills,
            update.state.dire.kills,
            update.state.gold_lead / 1000,
            market.team_a_odds * 100.0,
        );

        // Store in database
        match self.signal_store.insert_signal(&signal).await {
            Ok(id) => {
                info!("Stored snapshot id: {}", id);
            }
            Err(e) => {
                error!("Failed to store snapshot: {}", e);
            }
        }
    }
}
