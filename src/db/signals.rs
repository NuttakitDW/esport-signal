use std::str::FromStr;

use anyhow::{Context, Result};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Sqlite,
};
use tracing::info;

use crate::models::Signal;

/// SQLite store for match snapshots
pub struct SignalStore {
    pool: Pool<Sqlite>,
}

impl SignalStore {
    /// Create a new signal store and initialize the database
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

        info!("Signal store initialized");
        Ok(store)
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS signals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                market_condition_id TEXT NOT NULL,
                match_id INTEGER NOT NULL,
                market_team_a_odds REAL NOT NULL,
                match_snapshot TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create signals table")?;

        // Create indexes for common queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_signals_market
            ON signals (market_condition_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_signals_match
            ON signals (match_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_signals_created
            ON signals (created_at)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert a new signal
    pub async fn insert_signal(&self, signal: &Signal) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO signals (
                market_condition_id,
                match_id,
                market_team_a_odds,
                match_snapshot,
                created_at
            ) VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&signal.market_condition_id)
        .bind(signal.match_id)
        .bind(signal.market_team_a_odds)
        .bind(&signal.match_snapshot)
        .bind(signal.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to insert signal")?;

        Ok(result.last_insert_rowid())
    }

    /// Get recent signals for a market
    pub async fn get_signals_for_market(
        &self,
        market_condition_id: &str,
        limit: i64,
    ) -> Result<Vec<Signal>> {
        let rows = sqlx::query_as::<_, SignalRow>(
            r#"
            SELECT * FROM signals
            WHERE market_condition_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(market_condition_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch signals")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get recent signals for a match
    pub async fn get_signals_for_match(&self, match_id: i64, limit: i64) -> Result<Vec<Signal>> {
        let rows = sqlx::query_as::<_, SignalRow>(
            r#"
            SELECT * FROM signals
            WHERE match_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(match_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch signals")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get count of signals
    pub async fn get_signal_count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM signals")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count signals")?;

        Ok(row.0)
    }
}

/// Database row representation
#[derive(sqlx::FromRow)]
struct SignalRow {
    id: i64,
    market_condition_id: String,
    match_id: i64,
    market_team_a_odds: f64,
    match_snapshot: String,
    created_at: String,
}

impl From<SignalRow> for Signal {
    fn from(row: SignalRow) -> Self {
        Signal {
            id: Some(row.id),
            market_condition_id: row.market_condition_id,
            match_id: row.match_id,
            market_team_a_odds: row.market_team_a_odds,
            match_snapshot: row.match_snapshot,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}
