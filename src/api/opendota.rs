use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

/// Client for OpenDota REST API (historical data enrichment)
pub struct OpenDotaClient {
    client: Client,
    base_url: String,
}

/// Team information from OpenDota
#[derive(Debug, Clone, Deserialize)]
pub struct OpenDotaTeam {
    pub team_id: i64,
    pub name: String,
    pub tag: Option<String>,
    pub logo_url: Option<String>,
}

/// Match information from OpenDota
#[derive(Debug, Clone, Deserialize)]
pub struct OpenDotaMatch {
    pub match_id: i64,
    pub radiant_team_id: Option<i64>,
    pub dire_team_id: Option<i64>,
    pub radiant_win: Option<bool>,
    pub duration: Option<i32>,
    pub start_time: Option<i64>,
}

impl OpenDotaClient {
    /// Create a new OpenDota client
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    /// Search for teams by name
    pub async fn search_teams(&self, query: &str) -> Result<Vec<OpenDotaTeam>> {
        let url = format!("{}/search?q={}", self.base_url, urlencoding::encode(query));

        debug!("Searching OpenDota teams: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to search OpenDota teams")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenDota API error: {} - {}", status, text);
        }

        let teams: Vec<OpenDotaTeam> = response
            .json()
            .await
            .context("Failed to parse OpenDota search response")?;

        Ok(teams)
    }

    /// Get a team by ID
    pub async fn get_team(&self, team_id: i64) -> Result<Option<OpenDotaTeam>> {
        let url = format!("{}/teams/{}", self.base_url, team_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get OpenDota team")?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenDota API error: {} - {}", status, text);
        }

        let team: OpenDotaTeam = response
            .json()
            .await
            .context("Failed to parse OpenDota team response")?;

        Ok(Some(team))
    }

    /// Get recent matches for a team
    pub async fn get_team_matches(&self, team_id: i64, limit: usize) -> Result<Vec<OpenDotaMatch>> {
        let url = format!(
            "{}/teams/{}/matches?limit={}",
            self.base_url, team_id, limit
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get team matches")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenDota API error: {} - {}", status, text);
        }

        let matches: Vec<OpenDotaMatch> = response
            .json()
            .await
            .context("Failed to parse team matches response")?;

        Ok(matches)
    }

    /// Get match details by ID
    pub async fn get_match(&self, match_id: i64) -> Result<Option<OpenDotaMatch>> {
        let url = format!("{}/matches/{}", self.base_url, match_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get match")?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenDota API error: {} - {}", status, text);
        }

        let match_data: OpenDotaMatch = response
            .json()
            .await
            .context("Failed to parse match response")?;

        Ok(Some(match_data))
    }
}
