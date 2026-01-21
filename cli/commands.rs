mod add;
mod down;
mod init;
mod list;
mod lock;
mod package;
mod up;

use crate::commands::add::AddCommand;
use crate::commands::down::DownCommand;
use crate::commands::init::InitCommand;
use crate::commands::list::ListCommand;
use crate::commands::lock::LockCommand;
use crate::commands::package::PackageCommand;
use crate::commands::up::UpCommand;
use clap::{Parser, Subcommand, ValueEnum};
use std::process::ExitCode;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub(crate) enum OutputFormat {
    /// Output as a table
    Table,
    /// Output as JSON
    Json,
    /// Output as a simple list
    List,
}

trait ExecutableCommand {
    fn run(&self) -> u8;
}

/// A database migration tool for the rest of us.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub(crate) struct CLI {
    #[command(subcommand)]
    command: Command,
}

impl CLI {
    pub(crate) fn run() -> ExitCode {
        let instance = Self::parse();
        ExitCode::from(instance.command.run())
    }
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Initialize a new runway project
    Init(InitCommand),
    /// Lock a plan to prevent further changes
    Lock(LockCommand),
    /// Add a new change or plan
    Add(AddCommand),
    /// List changes or plans
    List(ListCommand),
    /// Package migrations into a distributable archive
    Package(PackageCommand),
    /// Apply migrations to the database
    Up(UpCommand),
    /// Revert migrations from the database
    Down(DownCommand),
}

impl ExecutableCommand for Command {
    fn run(&self) -> u8 {
        match self {
            Command::Add(command) => command.run(),
            Command::Init(command) => command.run(),
            Command::Lock(command) => command.run(),
            Command::List(command) => command.run(),
            Command::Package(command) => command.run(),
            Command::Up(command) => command.run(),
            Command::Down(command) => command.run(),
        }
    }
}
