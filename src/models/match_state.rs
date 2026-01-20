use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Live match state from OpenDota API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveMatchState {
    /// Match ID
    pub match_id: i64,

    /// League/tournament name
    pub league_name: Option<String>,

    /// Radiant team info
    pub radiant: TeamState,

    /// Dire team info
    pub dire: TeamState,

    /// Gold lead (radiant - dire, negative = dire leads)
    pub gold_lead: i64,

    /// Current game time in seconds
    pub game_time: i32,

    /// Whether the game is currently in progress
    pub is_live: bool,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// State of a team in a live match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamState {
    /// Team name
    pub name: String,

    /// Team ID (if known)
    pub team_id: Option<i64>,

    /// Current kill count
    pub kills: i32,

    /// Towers destroyed (enemy towers)
    pub towers_killed: i32,

    /// Barracks destroyed (enemy barracks)
    pub barracks_killed: i32,
}

impl Default for TeamState {
    fn default() -> Self {
        Self {
            name: String::new(),
            team_id: None,
            kills: 0,
            towers_killed: 0,
            barracks_killed: 0,
        }
    }
}

/// Update sent from Live Fetcher to Signal Processor
#[derive(Debug, Clone)]
pub struct MatchUpdate {
    /// Associated Polymarket condition_id
    pub market_condition_id: String,

    /// Current match state
    pub state: LiveMatchState,

    /// Previous state for diff calculation
    pub previous_state: Option<LiveMatchState>,
}

/// Map of match_id -> LiveMatchState for caching
pub type LiveMatchCache = std::collections::HashMap<i64, LiveMatchState>;
