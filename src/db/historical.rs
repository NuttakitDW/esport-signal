use std::str::FromStr;

use anyhow::{Context, Result};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Sqlite,
};
use tracing::info;

/// Historical match data for ML training
#[derive(Debug, Clone)]
pub struct HistoricalMatch {
    pub id: Option<i64>,
    pub match_id: i64,
    pub radiant_team: Option<String>,
    pub dire_team: Option<String>,
    pub radiant_win: bool,
    pub duration: i32,
    pub radiant_gold_adv: String,  // JSON array
    pub radiant_xp_adv: String,    // JSON array
    pub start_time: Option<i64>,
    pub league_name: Option<String>,
    pub fetched_at: String,
}

/// SQLite store for historical match data
pub struct HistoricalStore {
    pool: Pool<Sqlite>,
}

impl HistoricalStore {
    /// Create a new historical store and initialize the database
    pub async fn new(database_url: &str) -> Result<Self> {
        // Create data directory if needed
        if let Some(path) = database_url.strip_prefix("sqlite:") {
            if let Some(parent) = std::path::Path::new(path).parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .context("Failed to create database directory")?;
                }
            }
        }

        // Parse connection options and enable create_if_missing
        let options = SqliteConnectOptions::from_str(database_url)
            .context("Invalid database URL")?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .context("Failed to connect to database")?;

        let store = Self { pool };
        store.init_schema().await?;

        info!("Historical store initialized");
        Ok(store)
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS historical_matches (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                match_id INTEGER UNIQUE NOT NULL,
                radiant_team TEXT,
                dire_team TEXT,
                radiant_win BOOLEAN NOT NULL,
                duration INTEGER NOT NULL,
                radiant_gold_adv TEXT NOT NULL,
                radiant_xp_adv TEXT NOT NULL,
                start_time INTEGER,
                league_name TEXT,
                fetched_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create historical_matches table")?;

        // Create index on match_id for quick lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_historical_match_id
            ON historical_matches (match_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index on start_time for time-based queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_historical_start_time
            ON historical_matches (start_time)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert a new historical match
    pub async fn insert_match(&self, match_data: &HistoricalMatch) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO historical_matches (
                match_id,
                radiant_team,
                dire_team,
                radiant_win,
                duration,
                radiant_gold_adv,
                radiant_xp_adv,
                start_time,
                league_name,
                fetched_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(match_data.match_id)
        .bind(&match_data.radiant_team)
        .bind(&match_data.dire_team)
        .bind(match_data.radiant_win)
        .bind(match_data.duration)
        .bind(&match_data.radiant_gold_adv)
        .bind(&match_data.radiant_xp_adv)
        .bind(match_data.start_time)
        .bind(&match_data.league_name)
        .bind(&match_data.fetched_at)
        .execute(&self.pool)
        .await
        .context("Failed to insert historical match")?;

        Ok(result.last_insert_rowid())
    }

    /// Check if a match already exists
    pub async fn match_exists(&self, match_id: i64) -> Result<bool> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM historical_matches WHERE match_id = ?",
        )
        .bind(match_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to check match existence")?;

        Ok(row.0 > 0)
    }

    /// Get the count of historical matches
    pub async fn get_count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM historical_matches")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count historical matches")?;

        Ok(row.0)
    }

    /// Get the minimum match_id for pagination
    pub async fn get_min_match_id(&self) -> Result<Option<i64>> {
        let row: (Option<i64>,) = sqlx::query_as(
            "SELECT MIN(match_id) FROM historical_matches",
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to get min match_id")?;

        Ok(row.0)
    }

    /// Get all historical matches
    pub async fn get_all(&self) -> Result<Vec<HistoricalMatch>> {
        let rows = sqlx::query_as::<_, HistoricalMatchRow>(
            "SELECT * FROM historical_matches ORDER BY start_time DESC",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch historical matches")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

/// Database row representation
#[derive(sqlx::FromRow)]
struct HistoricalMatchRow {
    id: i64,
    match_id: i64,
    radiant_team: Option<String>,
    dire_team: Option<String>,
    radiant_win: bool,
    duration: i32,
    radiant_gold_adv: String,
    radiant_xp_adv: String,
    start_time: Option<i64>,
    league_name: Option<String>,
    fetched_at: String,
}

impl From<HistoricalMatchRow> for HistoricalMatch {
    fn from(row: HistoricalMatchRow) -> Self {
        HistoricalMatch {
            id: Some(row.id),
            match_id: row.match_id,
            radiant_team: row.radiant_team,
            dire_team: row.dire_team,
            radiant_win: row.radiant_win,
            duration: row.duration,
            radiant_gold_adv: row.radiant_gold_adv,
            radiant_xp_adv: row.radiant_xp_adv,
            start_time: row.start_time,
            league_name: row.league_name,
            fetched_at: row.fetched_at,
        }
    }
}
