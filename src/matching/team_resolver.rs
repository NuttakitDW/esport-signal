use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::models::{LiveMatchState, PolymarketMarket};

/// Resolves team names between Polymarket and live match data
pub struct TeamResolver {
    /// Map of alias -> canonical name
    aliases: HashMap<String, String>,
}

/// Team alias configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamAliases {
    pub teams: Vec<TeamAliasEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamAliasEntry {
    /// Canonical team name
    pub canonical: String,
    /// List of aliases (variations, abbreviations, etc.)
    pub aliases: Vec<String>,
}

/// Result of matching a market to a live match
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub market: PolymarketMarket,
    pub match_state: LiveMatchState,
    /// Which team in the market corresponds to radiant
    pub market_team_a_is_radiant: bool,
}

impl TeamResolver {
    /// Create a new resolver with no aliases
    pub fn new() -> Self {
        Self {
            aliases: HashMap::new(),
        }
    }

    /// Load aliases from a JSON file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).context("Failed to read team aliases file")?;

        let aliases_config: TeamAliases =
            serde_json::from_str(&content).context("Failed to parse team aliases JSON")?;

        let mut aliases = HashMap::new();

        for entry in aliases_config.teams {
            let canonical = entry.canonical.to_lowercase();

            // Map canonical name to itself
            aliases.insert(canonical.clone(), canonical.clone());

            // Map all aliases to canonical
            for alias in entry.aliases {
                aliases.insert(alias.to_lowercase(), canonical.clone());
            }
        }

        info!("Loaded {} team alias mappings", aliases.len());

        Ok(Self { aliases })
    }

    /// Normalize a team name to its canonical form
    pub fn normalize(&self, name: &str) -> String {
        let lower = name.to_lowercase().trim().to_string();

        self.aliases
            .get(&lower)
            .cloned()
            .unwrap_or_else(|| lower)
    }

    /// Check if two team names match (accounting for aliases)
    pub fn names_match(&self, name_a: &str, name_b: &str) -> bool {
        self.normalize(name_a) == self.normalize(name_b)
    }

    /// Find matching live matches for a market
    pub fn match_market_to_live(
        &self,
        market: &PolymarketMarket,
        live_matches: &[LiveMatchState],
    ) -> Option<MatchResult> {
        let market_team_a = self.normalize(&market.team_a);
        let market_team_b = self.normalize(&market.team_b);

        debug!(
            "Trying to match market: {} vs {}",
            market_team_a, market_team_b
        );

        for live_match in live_matches {
            let radiant_name = self.normalize(&live_match.radiant.name);
            let dire_name = self.normalize(&live_match.dire.name);

            debug!(
                "  Checking live match {}: {} vs {}",
                live_match.match_id, radiant_name, dire_name
            );

            // Check if market teams match live match teams
            // Team A could be either radiant or dire
            let team_a_is_radiant = market_team_a == radiant_name && market_team_b == dire_name;
            let team_a_is_dire = market_team_a == dire_name && market_team_b == radiant_name;

            if team_a_is_radiant || team_a_is_dire {
                info!(
                    "Matched market {} to live match {}",
                    market.condition_id, live_match.match_id
                );

                return Some(MatchResult {
                    market: market.clone(),
                    match_state: live_match.clone(),
                    market_team_a_is_radiant: team_a_is_radiant,
                });
            }
        }

        debug!("No match found for market {}", market.condition_id);
        None
    }

    /// Add a new alias mapping
    pub fn add_alias(&mut self, alias: &str, canonical: &str) {
        self.aliases
            .insert(alias.to_lowercase(), canonical.to_lowercase());
    }
}

impl Default for TeamResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        let mut resolver = TeamResolver::new();
        resolver.add_alias("ts", "team spirit");
        resolver.add_alias("spirit", "team spirit");

        assert_eq!(resolver.normalize("Team Spirit"), "team spirit");
        assert_eq!(resolver.normalize("TS"), "team spirit");
        assert_eq!(resolver.normalize("Spirit"), "team spirit");
        assert_eq!(resolver.normalize("OG"), "og"); // Unknown team stays as-is
    }

    #[test]
    fn test_names_match() {
        let mut resolver = TeamResolver::new();
        resolver.add_alias("ts", "team spirit");
        resolver.add_alias("spirit", "team spirit");

        assert!(resolver.names_match("Team Spirit", "TS"));
        assert!(resolver.names_match("Spirit", "Team Spirit"));
        assert!(!resolver.names_match("Team Spirit", "OG"));
    }
}
