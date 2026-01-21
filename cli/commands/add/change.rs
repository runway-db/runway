use clap::Args;
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub(crate) struct AddChangeCommand {
    /// Name of the change.
    pub name: String,
}

impl AddChangeCommand {
    pub fn run(&self, target_path: &PathBuf) -> u8 {
        let name = &self.name;
        let change_dir = target_path.join("changes").join(name);
        if change_dir.exists() {
            eprintln!(
                "Error: Change '{}' already exists at {:?}",
                name, change_dir
            );
            return 1;
        }

        if let Err(e) = fs::create_dir_all(&change_dir) {
            eprintln!("Error creating change directory {:?}: {}", change_dir, e);
            return 1;
        }

        if let Err(e) = fs::write(change_dir.join("change.toml"), "") {
            eprintln!("Error creating change.toml: {}", e);
            return 1;
        }

        if let Err(e) = fs::write(
            change_dir.join("deploy.sql"),
            "-- Add your deployment SQL here\n",
        ) {
            eprintln!("Error creating deploy.sql: {}", e);
            return 1;
        }

        println!("Added new change '{}' in {:?}", name, change_dir);
        0
    }
}
