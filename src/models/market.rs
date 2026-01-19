use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a Polymarket betting market for a Dota 2 match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketMarket {
    /// Unique market identifier (condition_id)
    pub condition_id: String,

    /// Market question/title (e.g., "Dota 2: Team Spirit vs OG (BO3)")
    pub question: String,

    /// Team A name extracted from market
    pub team_a: String,

    /// Team B name extracted from market
    pub team_b: String,

    /// Current odds for Team A (0.0 - 1.0)
    pub team_a_odds: f64,

    /// Current odds for Team B (0.0 - 1.0)
    pub team_b_odds: f64,

    /// Total liquidity in USD
    pub liquidity: f64,

    /// Market end time
    pub end_date: Option<DateTime<Utc>>,

    /// Whether the market is currently active
    pub active: bool,
}

/// Collection of active markets indexed by condition_id
pub type ActiveMarkets = std::collections::HashMap<String, PolymarketMarket>;
