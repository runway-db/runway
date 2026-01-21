use crate::commands::ExecutableCommand;
use crate::utils::source::load_source;
use clap::Parser;
use runway::{DatabaseEngine, Migrator, adapters};
use std::path::PathBuf;

/// Apply migrations to the database
#[derive(Parser)]
pub(crate) struct UpCommand {
    /// Path to the runway project or package (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    path: PathBuf,

    /// Database engine
    #[arg(short, long)]
    engine: DatabaseEngine,

    /// Database connection URL
    #[arg(short, long, env = "DATABASE_URL")]
    url: String,

    /// Target migration to apply to (inclusive)
    #[arg(short, long)]
    to: Option<String>,
}

impl ExecutableCommand for UpCommand {
    fn run(&self) -> u8 {
        let mut source = match load_source(&self.path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error loading source: {}", e);
                return 1;
            }
        };

        match self.engine {
            #[cfg(feature = "rusqlite")]
            DatabaseEngine::Sqlite => {
                let conn = match rusqlite::Connection::open(&self.url) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Error connecting to SQLite: {}", e);
                        return 1;
                    }
                };
                let adapter = adapters::Rusqlite::new(&conn);
                let mut migrator = Migrator::new(adapter, &mut source);
                if let Err(e) = migrator.apply_to(self.to.as_deref()) {
                    eprintln!("Error applying migrations: {}", e);
                    return 1;
                }
            }
            #[cfg(feature = "postgres")]
            DatabaseEngine::Postgres => {
                let mut client = match postgres::Client::connect(&self.url, postgres::NoTls) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Error connecting to Postgres: {}", e);
                        return 1;
                    }
                };
                let adapter = adapters::Postgres::new(&mut client);
                let mut migrator = Migrator::new(adapter, &mut source);
                if let Err(e) = migrator.apply_to(self.to.as_deref()) {
                    eprintln!("Error applying migrations: {}", e);
                    return 1;
                }
            }
            _ => {
                eprintln!("Unsupported engine: {}", self.engine);
                return 1;
            }
        }

        println!("Migrations applied successfully");
        0
    }
}
