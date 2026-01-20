use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

const OPENDOTA_BASE_URL: &str = "https://api.opendota.com/api";

/// Client for fetching historical match data from OpenDota
pub struct OpenDotaHistoricalClient {
    client: Client,
}

/// Pro match summary from /proMatches endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct ProMatch {
    pub match_id: i64,
    pub radiant_team_id: Option<i64>,
    pub radiant_name: Option<String>,
    pub dire_team_id: Option<i64>,
    pub dire_name: Option<String>,
    pub radiant_win: Option<bool>,
    pub duration: Option<i32>,
    pub start_time: Option<i64>,
    pub league_name: Option<String>,
}

/// Detailed match data from /matches/{id} endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct MatchDetails {
    pub match_id: i64,
    pub radiant_win: Option<bool>,
    pub duration: Option<i32>,
    pub start_time: Option<i64>,
    pub radiant_team: Option<TeamInfo>,
    pub dire_team: Option<TeamInfo>,
    pub league: Option<LeagueInfo>,
    pub radiant_gold_adv: Option<Vec<i32>>,
    pub radiant_xp_adv: Option<Vec<i32>>,
}

/// Team information in match details
#[derive(Debug, Clone, Deserialize)]
pub struct TeamInfo {
    pub team_id: Option<i64>,
    pub name: Option<String>,
    pub tag: Option<String>,
}

/// League information in match details
#[derive(Debug, Clone, Deserialize)]
pub struct LeagueInfo {
    pub leagueid: Option<i64>,
    pub name: Option<String>,
}

impl OpenDotaHistoricalClient {
    /// Create a new client
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Fetch list of pro matches, optionally paginated by less_than_match_id
    pub async fn get_pro_matches(&self, less_than_match_id: Option<i64>) -> Result<Vec<ProMatch>> {
        let url = match less_than_match_id {
            Some(id) => format!("{}/proMatches?less_than_match_id={}", OPENDOTA_BASE_URL, id),
            None => format!("{}/proMatches", OPENDOTA_BASE_URL),
        };

        debug!("Fetching pro matches: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch pro matches")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenDota API error: {} - {}", status, text);
        }

        let matches: Vec<ProMatch> = response
            .json()
            .await
            .context("Failed to parse pro matches response")?;

        Ok(matches)
    }

    /// Fetch detailed match data including gold/XP advantage arrays
    pub async fn get_match_details(&self, match_id: i64) -> Result<Option<MatchDetails>> {
        let url = format!("{}/matches/{}", OPENDOTA_BASE_URL, match_id);

        debug!("Fetching match details: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch match details")?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenDota API error: {} - {}", status, text);
        }

        let match_data: MatchDetails = response
            .json()
            .await
            .context("Failed to parse match details response")?;

        Ok(Some(match_data))
    }
}

impl Default for OpenDotaHistoricalClient {
    fn default() -> Self {
        Self::new()
    }
}
