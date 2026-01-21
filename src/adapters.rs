#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "rusqlite")]
mod rusqlite;

#[cfg(feature = "postgres")]
pub use postgres::PostgresAdapter as Postgres;
#[cfg(feature = "rusqlite")]
pub use rusqlite::RusqliteAdapter as Rusqlite;

pub struct AppliedMigration {
    pub name: String,
    pub type_name: String,
    pub hash: String,
    pub success: bool,
}

pub trait MigrationAdapter {
    const ENGINE: crate::DatabaseEngine;

    fn execute(&mut self, query: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn ensure_history_table(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn get_applied_migration(
        &mut self,
        name: &str,
    ) -> Result<Option<AppliedMigration>, Box<dyn std::error::Error>>;
    fn record_migration(
        &mut self,
        migration: &AppliedMigration,
        execution_time_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn mark_reverted(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>>;
}

#[allow(async_fn_in_trait)]
pub trait AsyncMigrationAdapter {
    const ENGINE: crate::DatabaseEngine;

    async fn execute(&mut self, query: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn ensure_history_table(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    async fn get_applied_migration(
        &mut self,
        name: &str,
    ) -> Result<Option<AppliedMigration>, Box<dyn std::error::Error>>;
    async fn record_migration(
        &mut self,
        migration: &AppliedMigration,
        execution_time_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>>;
    async fn mark_reverted(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>>;
}
