use std::sync::Arc;

use chrono::Utc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

use crate::db::SignalStore;
use crate::models::{
    ActiveMarkets, LiveMatchState, MatchUpdate, Signal, SignalStrength, SignalType,
};

/// Worker that processes match updates and generates signals
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

    /// Process a match update and generate signals
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

        // Detect signal type based on state changes
        let signal_type = self.detect_signal_type(&update.state, update.previous_state.as_ref());

        // Calculate win probability (simplified model)
        let team_a_win_prob = self.calculate_win_probability(&update.state);

        // Get current market odds
        let market_team_a_odds = market.team_a_odds;

        // Calculate edge
        let edge = team_a_win_prob - market_team_a_odds;

        // Calculate confidence based on game state
        let confidence = self.calculate_confidence(&update.state);

        // Determine signal strength
        let strength = SignalStrength::from_edge(edge);

        // Generate reason
        let reason = self.generate_reason(&update.state, &signal_type, edge);

        // Create signal
        let signal = Signal {
            id: None,
            market_condition_id: update.market_condition_id.clone(),
            match_id: update.state.match_id,
            signal_type,
            team_a_win_prob,
            market_team_a_odds,
            edge,
            confidence,
            strength,
            reason: reason.clone(),
            match_snapshot: serde_json::to_string(&update.state).unwrap_or_default(),
            created_at: Utc::now(),
        };

        // Log signal
        info!(
            "Signal: {} | Match {} | {} vs {} | Win Prob: {:.1}% | Market: {:.1}% | Edge: {:.1}% | {}",
            signal.signal_type.as_str(),
            signal.match_id,
            update.state.radiant.name,
            update.state.dire.name,
            team_a_win_prob * 100.0,
            market_team_a_odds * 100.0,
            edge * 100.0,
            signal.strength.as_str()
        );

        // Store in database
        match self.signal_store.insert_signal(&signal).await {
            Ok(id) => {
                info!("Signal stored with id: {}", id);
            }
            Err(e) => {
                error!("Failed to store signal: {}", e);
            }
        }
    }

    /// Detect the type of signal based on state changes
    fn detect_signal_type(
        &self,
        current: &LiveMatchState,
        previous: Option<&LiveMatchState>,
    ) -> SignalType {
        let previous = match previous {
            Some(p) => p,
            None => return SignalType::GameStart,
        };

        // Check for barracks kill (highest priority)
        let rax_diff = (current.radiant.barracks_killed - previous.radiant.barracks_killed)
            + (current.dire.barracks_killed - previous.dire.barracks_killed);
        if rax_diff > 0 {
            return SignalType::BarracksKill;
        }

        // Check for tower kill
        let tower_diff = (current.radiant.towers_killed - previous.radiant.towers_killed)
            + (current.dire.towers_killed - previous.dire.towers_killed);
        if tower_diff > 0 {
            return SignalType::TowerKill;
        }

        // Check for kill spree (5+ kills in the update)
        let kill_diff = (current.radiant.kills - previous.radiant.kills)
            + (current.dire.kills - previous.dire.kills);
        if kill_diff >= 5 {
            return SignalType::KillSpree;
        }

        // Check for large gold swing (5k+ change in net worth difference)
        let current_nw_diff =
            current.radiant.net_worth as i64 - current.dire.net_worth as i64;
        let previous_nw_diff =
            previous.radiant.net_worth as i64 - previous.dire.net_worth as i64;
        let nw_swing = (current_nw_diff - previous_nw_diff).abs();
        if nw_swing >= 5000 {
            return SignalType::GoldSwing;
        }

        // Check for late game (>35 min)
        if current.game_time > 2100 && previous.game_time <= 2100 {
            return SignalType::LateGame;
        }

        SignalType::PeriodicUpdate
    }

    /// Calculate win probability based on current state
    /// This is a simplified model - real implementation would be more sophisticated
    fn calculate_win_probability(&self, state: &LiveMatchState) -> f64 {
        let mut radiant_score = 0.5; // Start at 50%

        // Factor 1: Kill advantage (0.5% per kill)
        let kill_diff = state.radiant.kills - state.dire.kills;
        radiant_score += kill_diff as f64 * 0.005;

        // Factor 2: Gold advantage (1% per 1000 gold)
        let gold_diff = state.radiant.net_worth - state.dire.net_worth;
        radiant_score += (gold_diff as f64 / 1000.0) * 0.01;

        // Factor 3: Tower advantage (3% per tower)
        let tower_diff = state.radiant.towers_killed - state.dire.towers_killed;
        radiant_score += tower_diff as f64 * 0.03;

        // Factor 4: Barracks advantage (8% per barracks)
        let rax_diff = state.radiant.barracks_killed - state.dire.barracks_killed;
        radiant_score += rax_diff as f64 * 0.08;

        // Factor 5: Late game amplification
        let game_progress = (state.game_time as f64 / 2400.0).min(1.0); // 40 min = full progress
        let deviation_from_50 = radiant_score - 0.5;
        radiant_score = 0.5 + deviation_from_50 * (1.0 + game_progress * 0.5);

        // Clamp to valid probability range
        radiant_score.clamp(0.05, 0.95)
    }

    /// Calculate confidence in the signal
    fn calculate_confidence(&self, state: &LiveMatchState) -> f64 {
        let mut confidence = 0.5;

        // Higher confidence in later game (more data)
        let game_progress = (state.game_time as f64 / 2400.0).min(1.0);
        confidence += game_progress * 0.3;

        // Higher confidence with larger leads
        let kill_diff = (state.radiant.kills - state.dire.kills).abs();
        let gold_diff = (state.radiant.net_worth - state.dire.net_worth).abs();

        if kill_diff >= 10 || gold_diff >= 10000 {
            confidence += 0.15;
        }

        confidence.clamp(0.3, 0.95)
    }

    /// Generate human-readable reason for the signal
    fn generate_reason(
        &self,
        state: &LiveMatchState,
        signal_type: &SignalType,
        edge: f64,
    ) -> String {
        let direction = if edge > 0.0 {
            format!("{} favored", state.radiant.name)
        } else {
            format!("{} favored", state.dire.name)
        };

        let edge_pct = (edge.abs() * 100.0).round();

        match signal_type {
            SignalType::GameStart => format!("Game started: {} at {}%", direction, edge_pct),
            SignalType::KillSpree => format!(
                "Kill spree detected: {} ({}:{}) - {} at {}%",
                state.radiant.name,
                state.radiant.kills,
                state.dire.kills,
                direction,
                edge_pct
            ),
            SignalType::TowerKill => format!(
                "Tower destroyed: {} at {}%",
                direction, edge_pct
            ),
            SignalType::BarracksKill => format!(
                "Barracks destroyed: {} at {}%",
                direction, edge_pct
            ),
            SignalType::RoshanKill => format!(
                "Roshan killed: {} at {}%",
                direction, edge_pct
            ),
            SignalType::GoldSwing => {
                let gold_diff = state.radiant.net_worth - state.dire.net_worth;
                format!(
                    "Gold swing: {} lead by {}k - {} at {}%",
                    if gold_diff > 0 {
                        &state.radiant.name
                    } else {
                        &state.dire.name
                    },
                    (gold_diff.abs() as f64 / 1000.0).round(),
                    direction,
                    edge_pct
                )
            }
            SignalType::LateGame => format!(
                "Late game ({}min): {} at {}%",
                state.game_time / 60,
                direction,
                edge_pct
            ),
            SignalType::PeriodicUpdate => format!(
                "Update at {}:{:02}: {} at {}%",
                state.game_time / 60,
                state.game_time % 60,
                direction,
                edge_pct
            ),
        }
    }
}
