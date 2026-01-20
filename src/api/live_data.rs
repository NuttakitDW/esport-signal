use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, info};

use crate::models::{LiveMatchState, TeamState};

/// Client for live match data (using OpenDota API)
pub struct LiveDataClient {
    client: Client,
}

/// Live match from OpenDota API
#[derive(Debug, Deserialize)]
struct OpenDotaLiveMatch {
    match_id: String,
    league_id: i64,
    team_name_radiant: Option<String>,
    team_name_dire: Option<String>,
    team_id_radiant: Option<i64>,
    team_id_dire: Option<i64>,
    radiant_score: Option<i32>,
    dire_score: Option<i32>,
    radiant_lead: Option<i64>,
    game_time: Option<i32>,
    building_state: Option<i64>,
}

impl LiveDataClient {
    /// Create a new client
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Fetch all live professional matches using OpenDota API
    pub async fn fetch_live_matches(&self) -> Result<Vec<LiveMatchState>> {
        let url = "https://api.opendota.com/api/live";

        info!("Fetching live matches from OpenDota");

        let response = self
            .client
            .get(url)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to fetch live matches from OpenDota")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenDota API error: {} - {}", status, text);
        }

        let matches: Vec<OpenDotaLiveMatch> = response
            .json()
            .await
            .context("Failed to parse OpenDota live matches")?;

        // Filter for pro matches (league_id > 0 or has team names)
        let pro_matches: Vec<LiveMatchState> = matches
            .into_iter()
            .filter(|m| {
                m.league_id > 0
                    || m.team_name_radiant
                        .as_ref()
                        .map(|n| !n.is_empty())
                        .unwrap_or(false)
            })
            .map(|m| self.convert_match(m))
            .collect();

        info!("OpenDota returned {} live pro matches", pro_matches.len());

        Ok(pro_matches)
    }

    /// Fetch a specific match by ID
    pub async fn fetch_match(&self, match_id: i64) -> Result<Option<LiveMatchState>> {
        let matches = self.fetch_live_matches().await?;
        Ok(matches.into_iter().find(|m| m.match_id == match_id))
    }

    /// Convert OpenDota match to our model
    fn convert_match(&self, data: OpenDotaLiveMatch) -> LiveMatchState {
        let match_id: i64 = data.match_id.parse().unwrap_or(0);

        // Calculate building kills from building_state bitmask
        let (radiant_towers_killed, dire_towers_killed, radiant_rax_killed, dire_rax_killed) =
            self.parse_building_state(data.building_state);

        LiveMatchState {
            match_id,
            league_name: None, // OpenDota doesn't include league name in live data
            radiant: TeamState {
                name: data
                    .team_name_radiant
                    .unwrap_or_else(|| "Radiant".to_string()),
                team_id: data.team_id_radiant,
                kills: data.radiant_score.unwrap_or(0),
                towers_killed: dire_towers_killed,
                barracks_killed: dire_rax_killed,
            },
            dire: TeamState {
                name: data.team_name_dire.unwrap_or_else(|| "Dire".to_string()),
                team_id: data.team_id_dire,
                kills: data.dire_score.unwrap_or(0),
                towers_killed: radiant_towers_killed,
                barracks_killed: radiant_rax_killed,
            },
            gold_lead: data.radiant_lead.unwrap_or(0),
            game_time: data.game_time.unwrap_or(0),
            is_live: true,
            updated_at: Utc::now(),
        }
    }

    /// Parse building state bitmask
    /// Returns: (radiant_towers_killed, dire_towers_killed, radiant_rax_killed, dire_rax_killed)
    fn parse_building_state(&self, state: Option<i64>) -> (i32, i32, i32, i32) {
        let state = match state {
            Some(s) => s as u32,
            None => return (0, 0, 0, 0),
        };

        // Building state format (from OpenDota):
        // Bits 0-10: Radiant towers (11 towers)
        // Bits 11-16: Radiant barracks (6 barracks)
        // Bits 17-27: Dire towers (11 towers)
        // Bits 28-33: Dire barracks (6 barracks)

        let radiant_towers = state & 0x7FF; // bits 0-10
        let radiant_rax = (state >> 11) & 0x3F; // bits 11-16
        let dire_towers = (state >> 18) & 0x7FF; // bits 18-28
        let dire_rax = (state >> 29) & 0x3F; // bits 29-34

        // Count destroyed (0 bits = destroyed)
        let radiant_towers_destroyed = 11 - radiant_towers.count_ones() as i32;
        let dire_towers_destroyed = 11 - dire_towers.count_ones() as i32;
        let radiant_rax_destroyed = 6 - radiant_rax.count_ones() as i32;
        let dire_rax_destroyed = 6 - dire_rax.count_ones() as i32;

        (
            radiant_towers_destroyed,
            dire_towers_destroyed,
            radiant_rax_destroyed,
            dire_rax_destroyed,
        )
    }
}
