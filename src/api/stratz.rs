use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, warn};

use crate::models::{LiveMatchState, TeamState};

const STRATZ_GRAPHQL_URL: &str = "https://api.stratz.com/graphql";

/// Client for STRATZ GraphQL API
pub struct StratzClient {
    client: Client,
    api_token: String,
}

/// GraphQL response wrapper
#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

/// Live matches response structure
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveMatchesData {
    live: Option<LiveData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveData {
    matches: Option<Vec<LiveMatchData>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveMatchData {
    match_id: i64,
    game_time: Option<i32>,
    radiant_team: Option<TeamData>,
    dire_team: Option<TeamData>,
    league: Option<LeagueData>,
    radiant_score: Option<i32>,
    dire_score: Option<i32>,
    building_state: Option<BuildingState>,
    play_back_data: Option<PlaybackData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamData {
    id: Option<i64>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LeagueData {
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildingState {
    radiant_tower_state: Option<i32>,
    dire_tower_state: Option<i32>,
    radiant_barracks_state: Option<i32>,
    dire_barracks_state: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackData {
    radiant_net_worth: Option<i64>,
    dire_net_worth: Option<i64>,
}

impl StratzClient {
    /// Create a new STRATZ client
    pub fn new(api_token: &str) -> Self {
        Self {
            client: Client::new(),
            api_token: api_token.to_string(),
        }
    }

    /// Fetch all live professional matches
    pub async fn fetch_live_matches(&self) -> Result<Vec<LiveMatchState>> {
        let query = r#"
            query {
                live {
                    matches {
                        matchId
                        gameTime
                        radiantTeam {
                            id
                            name
                        }
                        direTeam {
                            id
                            name
                        }
                        league {
                            displayName
                        }
                        radiantScore
                        direScore
                        buildingState {
                            radiantTowerState
                            direTowerState
                            radiantBarracksState
                            direBarracksState
                        }
                        playBackData {
                            radiantNetWorth
                            direNetWorth
                        }
                    }
                }
            }
        "#;

        let response = self
            .client
            .post(STRATZ_GRAPHQL_URL)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "query": query
            }))
            .send()
            .await
            .context("Failed to fetch live matches from STRATZ")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("STRATZ API error: {} - {}", status, text);
        }

        let gql_response: GraphQLResponse<LiveMatchesData> = response
            .json()
            .await
            .context("Failed to parse STRATZ response")?;

        if let Some(errors) = gql_response.errors {
            if !errors.is_empty() {
                let error_msgs: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
                warn!("STRATZ GraphQL errors: {:?}", error_msgs);
            }
        }

        let matches = gql_response
            .data
            .and_then(|d| d.live)
            .and_then(|l| l.matches)
            .unwrap_or_default();

        let live_states: Vec<LiveMatchState> = matches
            .into_iter()
            .map(|m| self.convert_match_data(m))
            .collect();

        debug!("Fetched {} live matches", live_states.len());

        Ok(live_states)
    }

    /// Fetch a specific match by ID
    pub async fn fetch_match(&self, match_id: i64) -> Result<Option<LiveMatchState>> {
        let matches = self.fetch_live_matches().await?;
        Ok(matches.into_iter().find(|m| m.match_id == match_id))
    }

    /// Convert raw API data to our model
    fn convert_match_data(&self, data: LiveMatchData) -> LiveMatchState {
        let radiant_net_worth = data
            .play_back_data
            .as_ref()
            .and_then(|p| p.radiant_net_worth)
            .unwrap_or(0);

        let dire_net_worth = data
            .play_back_data
            .as_ref()
            .and_then(|p| p.dire_net_worth)
            .unwrap_or(0);

        // Calculate towers killed by counting destroyed towers in building state
        let (radiant_towers_killed, dire_towers_killed) =
            self.count_towers_killed(&data.building_state);

        let (radiant_rax_killed, dire_rax_killed) =
            self.count_barracks_killed(&data.building_state);

        LiveMatchState {
            match_id: data.match_id,
            league_name: data.league.and_then(|l| l.display_name),
            radiant: TeamState {
                name: data
                    .radiant_team
                    .as_ref()
                    .and_then(|t| t.name.clone())
                    .unwrap_or_else(|| "Radiant".to_string()),
                team_id: data.radiant_team.and_then(|t| t.id),
                kills: data.radiant_score.unwrap_or(0),
                net_worth: radiant_net_worth,
                xp_lead: 0, // Not directly available from this query
                towers_killed: dire_towers_killed,
                barracks_killed: dire_rax_killed,
                has_aegis: false, // Would need additional data
            },
            dire: TeamState {
                name: data
                    .dire_team
                    .as_ref()
                    .and_then(|t| t.name.clone())
                    .unwrap_or_else(|| "Dire".to_string()),
                team_id: data.dire_team.and_then(|t| t.id),
                kills: data.dire_score.unwrap_or(0),
                net_worth: dire_net_worth,
                xp_lead: 0,
                towers_killed: radiant_towers_killed,
                barracks_killed: radiant_rax_killed,
                has_aegis: false,
            },
            game_time: data.game_time.unwrap_or(0),
            is_live: true,
            updated_at: Utc::now(),
        }
    }

    /// Count destroyed towers from building state bitmask
    fn count_towers_killed(&self, state: &Option<BuildingState>) -> (i32, i32) {
        let state = match state {
            Some(s) => s,
            None => return (0, 0),
        };

        // Tower state is a bitmask - count zeros (destroyed towers)
        // Full tower state = 2047 (11 towers per side)
        let radiant_state = state.radiant_tower_state.unwrap_or(2047);
        let dire_state = state.dire_tower_state.unwrap_or(2047);

        let radiant_destroyed = 11 - radiant_state.count_ones() as i32;
        let dire_destroyed = 11 - dire_state.count_ones() as i32;

        (radiant_destroyed, dire_destroyed)
    }

    /// Count destroyed barracks from building state bitmask
    fn count_barracks_killed(&self, state: &Option<BuildingState>) -> (i32, i32) {
        let state = match state {
            Some(s) => s,
            None => return (0, 0),
        };

        // Barracks state is a bitmask - 6 barracks per side (3 melee + 3 ranged)
        let radiant_state = state.radiant_barracks_state.unwrap_or(63);
        let dire_state = state.dire_barracks_state.unwrap_or(63);

        let radiant_destroyed = 6 - radiant_state.count_ones() as i32;
        let dire_destroyed = 6 - dire_state.count_ones() as i32;

        (radiant_destroyed, dire_destroyed)
    }
}
