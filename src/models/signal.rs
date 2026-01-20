use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A betting signal generated from live match analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Unique signal identifier
    pub id: Option<i64>,

    /// Polymarket condition_id this signal relates to
    pub market_condition_id: String,

    /// Match ID from STRATZ/Valve
    pub match_id: i64,

    /// Signal type
    pub signal_type: SignalType,

    /// Predicted win probability for team A (radiant)
    pub team_a_win_prob: f64,

    /// Current market odds for team A
    pub market_team_a_odds: f64,

    /// Edge: difference between predicted and market odds
    pub edge: f64,

    /// Confidence level (0.0 - 1.0)
    pub confidence: f64,

    /// Signal strength (based on edge and confidence)
    pub strength: SignalStrength,

    /// Human-readable reason for the signal
    pub reason: String,

    /// Raw match data at signal time (JSON)
    pub match_snapshot: String,

    /// When the signal was generated
    pub created_at: DateTime<Utc>,
}

/// Type of signal event
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    /// Regular periodic update
    PeriodicUpdate,
    /// First kill of the game
    FirstBlood,
    /// Significant kill event (teamfight)
    KillSpree,
    /// Tower destroyed
    TowerKill,
    /// Barracks destroyed
    BarracksKill,
    /// Roshan killed / Aegis claimed
    RoshanKill,
    /// Large gold swing
    GoldSwing,
    /// Game start
    GameStart,
    /// Game approaching end (high ground siege)
    LateGame,
}

impl SignalType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SignalType::PeriodicUpdate => "periodic_update",
            SignalType::FirstBlood => "first_blood",
            SignalType::KillSpree => "kill_spree",
            SignalType::TowerKill => "tower_kill",
            SignalType::BarracksKill => "barracks_kill",
            SignalType::RoshanKill => "roshan_kill",
            SignalType::GoldSwing => "gold_swing",
            SignalType::GameStart => "game_start",
            SignalType::LateGame => "late_game",
        }
    }
}

/// Signal strength classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignalStrength {
    /// Edge < 3%
    Weak,
    /// Edge 3-7%
    Moderate,
    /// Edge 7-12%
    Strong,
    /// Edge > 12%
    VeryStrong,
}

impl SignalStrength {
    pub fn from_edge(edge: f64) -> Self {
        let abs_edge = edge.abs();
        if abs_edge < 0.03 {
            SignalStrength::Weak
        } else if abs_edge < 0.07 {
            SignalStrength::Moderate
        } else if abs_edge < 0.12 {
            SignalStrength::Strong
        } else {
            SignalStrength::VeryStrong
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SignalStrength::Weak => "weak",
            SignalStrength::Moderate => "moderate",
            SignalStrength::Strong => "strong",
            SignalStrength::VeryStrong => "very_strong",
        }
    }
}
