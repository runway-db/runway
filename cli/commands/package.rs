use crate::commands::ExecutableCommand;
use clap::Parser;
use std::path::PathBuf;

/// Package migrations into a distributable archive
#[derive(Parser)]
pub(crate) struct PackageCommand {
    /// Path to the runway project (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    path: PathBuf,

    /// Output path for the generated package
    #[arg(short, long)]
    output: PathBuf,

    /// Create an exploded package (a directory instead of a ZIP file)
    #[arg(short, long)]
    exploded: bool,
}

impl ExecutableCommand for PackageCommand {
    fn run(&self) -> u8 {
        match runway::package_migrations(&self.path, &self.output, self.exploded) {
            Ok(_) => {
                println!("Package created successfully at {:?}", self.output);
                0
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                1
            }
        }
    }
}
