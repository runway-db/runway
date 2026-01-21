use crate::DatabaseEngine;
use crate::adapters::{AppliedMigration, MigrationAdapter};

pub struct RusqliteAdapter<'a> {
    connection: &'a rusqlite::Connection,
}

impl<'a> RusqliteAdapter<'a> {
    pub fn new(conn: &'a rusqlite::Connection) -> Self {
        Self { connection: conn }
    }
}

impl MigrationAdapter for RusqliteAdapter<'_> {
    const ENGINE: DatabaseEngine = DatabaseEngine::Sqlite;

    fn execute(&mut self, query: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.connection.execute_batch(query)?;
        Ok(())
    }

    fn ensure_history_table(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let query = "
            CREATE TABLE IF NOT EXISTS _runway_history (
                id              INTEGER PRIMARY KEY,
                name            TEXT NOT NULL,
                type            TEXT NOT NULL,
                hash            TEXT NOT NULL,
                applied_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
                success         BOOLEAN NOT NULL,
                execution_time  INTEGER,
                reverted_at     DATETIME NULL,
                runway_version  TEXT NOT NULL
            );
        ";
        self.execute(query)
    }

    fn get_applied_migration(
        &mut self,
        name: &str,
    ) -> Result<Option<AppliedMigration>, Box<dyn std::error::Error>> {
        let mut stmt = self.connection.prepare(
            "SELECT name, type, hash, success FROM _runway_history WHERE name = ? AND reverted_at IS NULL ORDER BY applied_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query([name])?;

        if let Some(row) = rows.next()? {
            Ok(Some(AppliedMigration {
                name: row.get(0)?,
                type_name: row.get(1)?,
                hash: row.get(2)?,
                success: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn record_migration(
        &mut self,
        migration: &AppliedMigration,
        execution_time_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.connection.execute(
            "INSERT INTO _runway_history (name, type, hash, success, execution_time, runway_version) VALUES (?, ?, ?, ?, ?, ?)",
            (
                &migration.name,
                &migration.type_name,
                &migration.hash,
                migration.success,
                execution_time_ms as i64,
                env!("CARGO_PKG_VERSION"),
            ),
        )?;
        Ok(())
    }

    fn mark_reverted(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.connection.execute(
            "UPDATE _runway_history SET reverted_at = CURRENT_TIMESTAMP WHERE name = ? AND success = 1 AND reverted_at IS NULL",
            [name],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn create_adapter() {
        let _conn = rusqlite::Connection::open_in_memory().unwrap();
    }
}
