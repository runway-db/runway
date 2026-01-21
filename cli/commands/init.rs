use crate::commands::ExecutableCommand;
use clap::Args;
use std::fs;
use std::path::PathBuf;

/// Initialize a new runway project structure.
#[derive(Args)]
pub(crate) struct InitCommand {
    /// Path to scaffold out a runway project structure.
    path: Option<PathBuf>,
}

impl ExecutableCommand for InitCommand {
    fn run(&self) -> u8 {
        let target_path = self.path.clone().unwrap_or_else(|| PathBuf::from("."));

        if target_path.exists() {
            if !target_path.is_dir() {
                eprintln!(
                    "Error: Path {:?} exists and is not a directory",
                    target_path
                );
                return 1;
            }
            match fs::read_dir(&target_path) {
                Ok(mut entries) => {
                    if entries.next().is_some() {
                        eprintln!("Error: Directory {:?} is not empty", target_path);
                        return 1;
                    }
                }
                Err(e) => {
                    eprintln!("Error reading directory {:?}: {}", target_path, e);
                    return 1;
                }
            }
        } else {
            if let Err(e) = fs::create_dir_all(&target_path) {
                eprintln!("Error creating directory {:?}: {}", target_path, e);
                return 1;
            }
        }

        // Create empty project structure
        let runway_toml = target_path.join("runway.toml");
        let changes_dir = target_path.join("changes");
        let plans_dir = target_path.join("plans");

        if let Err(e) = fs::write(&runway_toml, "") {
            eprintln!("Error creating runway.toml: {}", e);
            return 1;
        }

        if let Err(e) = fs::create_dir(&changes_dir) {
            eprintln!("Error creating changes/ directory: {}", e);
            return 1;
        }

        if let Err(e) = fs::create_dir(&plans_dir) {
            eprintln!("Error creating plans/ directory: {}", e);
            return 1;
        }

        println!("Initialized empty runway project in {:?}", target_path);
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn test_init_new_dir() {
        let tmp_dir = TempDir::new("runway_test").unwrap();
        let project_dir = tmp_dir.path().join("my_project");
        let cmd = InitCommand {
            path: Some(project_dir.clone()),
        };

        assert_eq!(cmd.run(), 0);
        assert!(project_dir.join("runway.toml").exists());
        assert!(project_dir.join("changes").is_dir());
        assert!(project_dir.join("plans").is_dir());
    }

    #[test]
    fn test_init_existing_empty_dir() {
        let tmp_dir = TempDir::new("runway_test").unwrap();
        let project_dir = tmp_dir.path().join("my_project");
        fs::create_dir_all(&project_dir).unwrap();

        let cmd = InitCommand {
            path: Some(project_dir.clone()),
        };

        assert_eq!(cmd.run(), 0);
        assert!(project_dir.join("runway.toml").exists());
    }

    #[test]
    fn test_init_existing_non_empty_dir() {
        let tmp_dir = TempDir::new("runway_test").unwrap();
        let project_dir = tmp_dir.path().join("my_project");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(project_dir.join("some_file"), "data").unwrap();

        let cmd = InitCommand {
            path: Some(project_dir.clone()),
        };

        assert_eq!(cmd.run(), 1);
        assert!(!project_dir.join("runway.toml").exists());
    }
}
