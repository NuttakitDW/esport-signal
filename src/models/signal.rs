use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A match snapshot captured during live monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Unique signal identifier
    pub id: Option<i64>,

    /// Polymarket condition_id this signal relates to
    pub market_condition_id: String,

    /// Match ID from OpenDota
    pub match_id: i64,

    /// Current market odds for team A (from Polymarket)
    pub market_team_a_odds: f64,

    /// Raw match data at signal time (JSON)
    pub match_snapshot: String,

    /// When the signal was generated
    pub created_at: DateTime<Utc>,
}
