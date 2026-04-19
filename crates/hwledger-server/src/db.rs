//! SQLite database initialization and schema.
//!
//! Tables: agents, devices, telemetry, jobs.
//! Traces to: FR-FLEET-001

use anyhow::Result;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;
use tracing::info;

/// Initialize SQLite pool and run migrations.
/// Traces to: FR-FLEET-001
pub async fn init(db_path: &Path) -> Result<SqlitePool> {
    // Create database file if it doesn't exist
    if !db_path.exists() {
        std::fs::File::create(db_path)?;
    }

    let database_url = format!("sqlite://{}", db_path.display());
    let pool = SqlitePoolOptions::new().max_connections(10).connect(&database_url).await?;

    // Run inline migrations
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            hostname TEXT NOT NULL,
            platform_json TEXT NOT NULL,
            cert_pem TEXT NOT NULL,
            registered_at_ms INTEGER NOT NULL,
            last_seen_ms INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS devices (
            agent_id TEXT NOT NULL,
            device_idx INTEGER NOT NULL,
            backend TEXT NOT NULL,
            name TEXT NOT NULL,
            uuid TEXT,
            total_vram_bytes INTEGER NOT NULL,
            PRIMARY KEY (agent_id, device_idx),
            FOREIGN KEY (agent_id) REFERENCES agents(id)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS telemetry (
            agent_id TEXT NOT NULL,
            device_idx INTEGER NOT NULL,
            captured_at_ms INTEGER NOT NULL,
            free_vram_bytes INTEGER,
            util_percent REAL,
            temperature_c REAL,
            power_watts REAL,
            PRIMARY KEY (agent_id, device_idx, captured_at_ms),
            FOREIGN KEY (agent_id, device_idx) REFERENCES devices(agent_id, device_idx)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            model_ref TEXT NOT NULL,
            state TEXT NOT NULL,
            started_at_ms INTEGER,
            finished_at_ms INTEGER,
            exit_code INTEGER,
            log_tail TEXT,
            created_at_ms INTEGER NOT NULL,
            FOREIGN KEY (agent_id) REFERENCES agents(id)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    info!("Database schema initialized");
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Traces to: FR-FLEET-001
    #[tokio::test]
    async fn test_db_init() {
        let temp = TempDir::new().expect("create temp dir");
        let db_path = temp.path().join("test.db");
        let pool = init(&db_path).await.expect("init db");
        assert!(db_path.exists());

        // Verify tables exist
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents")
            .fetch_one(&pool)
            .await
            .expect("query agents");
        assert_eq!(row.0, 0);
    }
}
