use crate::DatabaseEngine;
use crate::adapters::{AppliedMigration, MigrationAdapter};
pub use postgres::Client;

pub struct PostgresAdapter<'a> {
    client: &'a mut Client,
}

impl<'a> PostgresAdapter<'a> {
    pub fn new(client: &'a mut Client) -> Self {
        Self { client }
    }
}

impl MigrationAdapter for PostgresAdapter<'_> {
    const ENGINE: DatabaseEngine = DatabaseEngine::Postgres;

    fn execute(&mut self, query: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client.batch_execute(query)?;
        Ok(())
    }

    fn ensure_history_table(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let query = "
            CREATE TABLE IF NOT EXISTS _runway_history (
                id              SERIAL PRIMARY KEY,
                name            TEXT NOT NULL,
                type            TEXT NOT NULL,
                hash            TEXT NOT NULL,
                applied_at      TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                success         BOOLEAN NOT NULL,
                execution_time  INTEGER,
                reverted_at     TIMESTAMPTZ NULL,
                runway_version  TEXT NOT NULL
            );
        ";
        self.execute(query)
    }

    fn get_applied_migration(
        &mut self,
        name: &str,
    ) -> Result<Option<AppliedMigration>, Box<dyn std::error::Error>> {
        let row = self.client.query_opt(
            "SELECT name, type, hash, success FROM _runway_history WHERE name = $1 AND reverted_at IS NULL ORDER BY applied_at DESC LIMIT 1",
            &[&name],
        )?;

        match row {
            Some(row) => Ok(Some(AppliedMigration {
                name: row.get(0),
                type_name: row.get(1),
                hash: row.get(2),
                success: row.get(3),
            })),
            None => Ok(None),
        }
    }

    fn record_migration(
        &mut self,
        migration: &AppliedMigration,
        execution_time_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.client.execute(
            "INSERT INTO _runway_history (name, type, hash, success, execution_time, runway_version) VALUES ($1, $2, $3, $4, $5, $6)",
            &[
                &migration.name,
                &migration.type_name,
                &migration.hash,
                &migration.success,
                &(execution_time_ms as i32),
                &env!("CARGO_PKG_VERSION"),
            ],
        )?;
        Ok(())
    }

    fn mark_reverted(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client.execute(
            "UPDATE _runway_history SET reverted_at = CURRENT_TIMESTAMP WHERE name = $1 AND success = true AND reverted_at IS NULL",
            &[&name],
        )?;
        Ok(())
    }
}
