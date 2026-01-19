use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::models::PolymarketMarket;

const DOTA2_SERIES_ID: &str = "10309";

/// Client for Polymarket Gamma API
pub struct PolymarketClient {
    client: Client,
    base_url: String,
}

/// Series response from Polymarket (events list only)
#[derive(Debug, Deserialize)]
struct SeriesResponse {
    events: Vec<SeriesEvent>,
}

/// Event in series list (minimal info)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeriesEvent {
    id: String,
    active: bool,
    closed: bool,
}

/// Full event response with markets
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventResponse {
    id: String,
    title: String,
    active: bool,
    closed: bool,
    #[serde(default)]
    markets: Vec<MarketResponse>,
}

/// Individual market within an event
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarketResponse {
    condition_id: String,
    question: String,
    outcomes: String,
    outcome_prices: String,
    liquidity: Option<String>,
    liquidity_num: Option<f64>,
    active: bool,
    closed: bool,
    end_date_iso: Option<String>,
    #[serde(default)]
    sports_market_type: Option<String>,
}

impl PolymarketClient {
    /// Create a new Polymarket client
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    /// Fetch active Dota 2 markets from Polymarket sports series
    pub async fn fetch_dota2_markets(&self) -> Result<Vec<PolymarketMarket>> {
        // Step 1: Get list of events from series
        let series_url = format!("{}/series/{}", self.base_url, DOTA2_SERIES_ID);
        debug!("Fetching Dota 2 series from: {}", series_url);

        let response = self
            .client
            .get(&series_url)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to fetch Dota 2 series")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            warn!("Polymarket API error: {} - {}", status, text);
            return Ok(Vec::new());
        }

        let series: SeriesResponse = response
            .json()
            .await
            .context("Failed to parse Dota 2 series response")?;

        // Step 2: Filter active events and fetch each one for markets
        let active_event_ids: Vec<String> = series
            .events
            .into_iter()
            .filter(|e| e.active && !e.closed)
            .map(|e| e.id)
            .collect();

        debug!("Found {} active events", active_event_ids.len());

        let mut markets = Vec::new();

        // Fetch each event to get its markets
        for event_id in active_event_ids {
            match self.fetch_event_markets(&event_id).await {
                Ok(event_markets) => markets.extend(event_markets),
                Err(e) => {
                    warn!("Failed to fetch event {}: {}", event_id, e);
                }
            }
        }

        info!("Total active Dota 2 markets found: {}", markets.len());
        Ok(markets)
    }

    /// Fetch markets for a specific event
    async fn fetch_event_markets(&self, event_id: &str) -> Result<Vec<PolymarketMarket>> {
        let url = format!("{}/events/{}", self.base_url, event_id);
        debug!("Fetching event: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to fetch event")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Event API error: {} - {}", status, text);
        }

        let event: EventResponse = response
            .json()
            .await
            .context("Failed to parse event response")?;

        if !event.active || event.closed {
            return Ok(Vec::new());
        }

        let mut markets = Vec::new();

        for market in event.markets {
            // Only include moneyline markets (series winner)
            let is_moneyline = market
                .sports_market_type
                .as_ref()
                .map(|t| t == "moneyline")
                .unwrap_or(false);

            if !is_moneyline || !market.active || market.closed {
                continue;
            }

            if let Some(pm) = self.convert_market(market) {
                info!(
                    "Found market: {} vs {} (odds: {:.0}% / {:.0}%)",
                    pm.team_a,
                    pm.team_b,
                    pm.team_a_odds * 100.0,
                    pm.team_b_odds * 100.0
                );
                markets.push(pm);
            }
        }

        Ok(markets)
    }

    /// Convert API market response to our model
    fn convert_market(&self, market: MarketResponse) -> Option<PolymarketMarket> {
        // Parse JSON string arrays
        let outcomes: Vec<String> = serde_json::from_str(&market.outcomes).ok()?;
        let outcome_prices: Vec<String> = serde_json::from_str(&market.outcome_prices).ok()?;

        // Need exactly 2 outcomes for a match winner market
        if outcomes.len() != 2 || outcome_prices.len() != 2 {
            return None;
        }

        let team_a = outcomes.get(0)?.trim().to_string();
        let team_b = outcomes.get(1)?.trim().to_string();

        let team_a_odds: f64 = outcome_prices.get(0)?.parse().ok()?;
        let team_b_odds: f64 = outcome_prices.get(1)?.parse().ok()?;

        let liquidity: f64 = market
            .liquidity_num
            .or_else(|| market.liquidity.as_ref().and_then(|l| l.parse().ok()))
            .unwrap_or(0.0);

        let end_date = market
            .end_date_iso
            .as_ref()
            .and_then(|d| {
                chrono::DateTime::parse_from_rfc3339(d)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .or_else(|| {
                        chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
                            .ok()
                            .map(|date| date.and_hms_opt(0, 0, 0).unwrap().and_utc())
                    })
            });

        Some(PolymarketMarket {
            condition_id: market.condition_id,
            question: market.question,
            team_a,
            team_b,
            team_a_odds,
            team_b_odds,
            liquidity,
            end_date,
            active: market.active && !market.closed,
        })
    }
}
