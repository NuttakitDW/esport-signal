use std::env;
use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use tokio::time::sleep;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use esport_signal::api::opendota_historical::{OpenDotaHistoricalClient, ProMatch};
use esport_signal::db::historical::{HistoricalMatch, HistoricalStore};

const DEFAULT_COUNT: usize = 1000;
const RATE_LIMIT_DELAY: Duration = Duration::from_millis(1100); // Slightly over 1 second
const PROGRESS_INTERVAL: usize = 10;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "fetch_historical=info,esport_signal=info,warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse arguments
    let args: Vec<String> = env::args().collect();
    let target_count = parse_count(&args);

    info!("Fetching {} historical pro matches from OpenDota", target_count);

    // Initialize database
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:data/signals.db".to_string());

    let store = HistoricalStore::new(&database_url).await?;
    let client = OpenDotaHistoricalClient::new();

    // Check existing count
    let existing_count = store.get_count().await? as usize;
    info!("Found {} existing historical matches in database", existing_count);

    if existing_count >= target_count {
        info!("Already have {} matches, target is {}. Nothing to fetch.", existing_count, target_count);
        return Ok(());
    }

    let matches_needed = target_count - existing_count;
    info!("Need to fetch {} more matches", matches_needed);

    // Get starting point for pagination
    let mut less_than_match_id = store.get_min_match_id().await?;
    if less_than_match_id.is_some() {
        info!("Resuming from match_id < {}", less_than_match_id.unwrap());
    }

    let mut fetched_count = 0;
    let mut skipped_count = 0;
    let mut failed_count = 0;

    while fetched_count < matches_needed {
        // Fetch batch of pro matches
        let pro_matches = match client.get_pro_matches(less_than_match_id).await {
            Ok(matches) => matches,
            Err(e) => {
                error!("Failed to fetch pro matches: {}", e);
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        if pro_matches.is_empty() {
            info!("No more pro matches available");
            break;
        }

        info!("Got {} pro matches from list", pro_matches.len());

        // Process each match
        for pro_match in &pro_matches {
            if fetched_count >= matches_needed {
                break;
            }

            // Update pagination cursor
            less_than_match_id = Some(pro_match.match_id);

            // Skip if already exists
            if store.match_exists(pro_match.match_id).await? {
                skipped_count += 1;
                continue;
            }

            // Rate limit
            sleep(RATE_LIMIT_DELAY).await;

            // Fetch detailed match data
            match fetch_and_store_match(&client, &store, pro_match).await {
                Ok(true) => {
                    fetched_count += 1;

                    // Progress update
                    if fetched_count % PROGRESS_INTERVAL == 0 {
                        let total = existing_count + fetched_count;
                        info!(
                            "Progress: {}/{} fetched ({} total in DB, {} skipped, {} failed)",
                            fetched_count, matches_needed, total, skipped_count, failed_count
                        );
                    }
                }
                Ok(false) => {
                    // Match didn't have required data, skip
                    skipped_count += 1;
                }
                Err(e) => {
                    warn!("Failed to fetch match {}: {}", pro_match.match_id, e);
                    failed_count += 1;

                    // Extra delay on failure
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }

        // Small delay between batches
        sleep(RATE_LIMIT_DELAY).await;
    }

    let final_count = store.get_count().await?;
    info!("Completed! Total matches in database: {}", final_count);
    info!(
        "Session: {} fetched, {} skipped (existing or incomplete), {} failed",
        fetched_count, skipped_count, failed_count
    );

    Ok(())
}

/// Parse --count argument
fn parse_count(args: &[String]) -> usize {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--count" || arg == "-c" {
            if let Some(value) = args.get(i + 1) {
                if let Ok(count) = value.parse() {
                    return count;
                }
            }
        }
    }
    DEFAULT_COUNT
}

/// Fetch detailed match data and store in database
async fn fetch_and_store_match(
    client: &OpenDotaHistoricalClient,
    store: &HistoricalStore,
    pro_match: &ProMatch,
) -> Result<bool> {
    let details = match client.get_match_details(pro_match.match_id).await? {
        Some(d) => d,
        None => {
            warn!("Match {} not found", pro_match.match_id);
            return Ok(false);
        }
    };

    // Skip matches without gold/XP data (required for ML training)
    let radiant_gold_adv = match &details.radiant_gold_adv {
        Some(arr) if !arr.is_empty() => serde_json::to_string(arr)?,
        _ => {
            warn!("Match {} has no gold advantage data", pro_match.match_id);
            return Ok(false);
        }
    };

    let radiant_xp_adv = match &details.radiant_xp_adv {
        Some(arr) if !arr.is_empty() => serde_json::to_string(arr)?,
        _ => {
            warn!("Match {} has no XP advantage data", pro_match.match_id);
            return Ok(false);
        }
    };

    // Extract team names
    let radiant_team = details
        .radiant_team
        .as_ref()
        .and_then(|t| t.name.clone())
        .or_else(|| pro_match.radiant_name.clone());

    let dire_team = details
        .dire_team
        .as_ref()
        .and_then(|t| t.name.clone())
        .or_else(|| pro_match.dire_name.clone());

    // Extract league name
    let league_name = details
        .league
        .as_ref()
        .and_then(|l| l.name.clone())
        .or_else(|| pro_match.league_name.clone());

    // Build historical match record
    let historical_match = HistoricalMatch {
        id: None,
        match_id: details.match_id,
        radiant_team,
        dire_team,
        radiant_win: details.radiant_win.unwrap_or(false),
        duration: details.duration.unwrap_or(0),
        radiant_gold_adv,
        radiant_xp_adv,
        start_time: details.start_time,
        league_name,
        fetched_at: Utc::now().to_rfc3339(),
    };

    store.insert_match(&historical_match).await?;

    Ok(true)
}
