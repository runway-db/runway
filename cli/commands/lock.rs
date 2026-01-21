use crate::commands::ExecutableCommand;
use clap::Args;
use runway::load_project;

use std::path::PathBuf;

/// Lock a plan to prevent further changes to its included changes.
#[derive(Args)]
pub struct LockCommand {
    /// Name of the plan to lock.
    plan: String,

    /// Path to the runway project.
    #[arg(short, long)]
    path: Option<PathBuf>,
}

impl ExecutableCommand for LockCommand {
    fn run(&self) -> u8 {
        let target_path = self.path.clone().unwrap_or_else(|| PathBuf::from("."));
        let mut project = match load_project(&target_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to load project: {e}");
                return 1;
            }
        };

        match project.lock_plan(&self.plan) {
            Ok(_) => {
                println!("Successfully locked plan '{}'", self.plan);
                0
            }
            Err(e) => {
                eprintln!("Failed to lock plan '{}': {e}", self.plan);
                1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempdir::TempDir;

    fn setup_test_project() -> TempDir {
        let tmp_dir = TempDir::new("runway_lock_test").unwrap();
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("basic");

        fn copy_dir_all(
            src: impl AsRef<std::path::Path>,
            dst: impl AsRef<std::path::Path>,
        ) -> std::io::Result<()> {
            fs::create_dir_all(&dst)?;
            for entry in fs::read_dir(src)? {
                let entry = entry?;
                let ty = entry.file_type()?;
                if ty.is_dir() {
                    copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
                } else {
                    fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
                }
            }
            Ok(())
        }

        copy_dir_all(fixture_path, tmp_dir.path()).unwrap();
        tmp_dir
    }

    #[test]
    fn test_lock_plan() {
        let tmp_dir = setup_test_project();
        let cmd = LockCommand {
            plan: "p1".to_string(),
            path: Some(tmp_dir.path().to_path_buf()),
        };

        assert_eq!(cmd.run(), 0);
        assert!(tmp_dir.path().join("plans/p1/plan.lock").exists());
    }

    #[test]
    fn test_lock_non_existent_plan() {
        let tmp_dir = setup_test_project();
        let cmd = LockCommand {
            plan: "non_existent".to_string(),
            path: Some(tmp_dir.path().to_path_buf()),
        };

        assert_eq!(cmd.run(), 1);
    }

    #[test]
    fn test_lock_non_existent_project() {
        let tmp_dir = TempDir::new("runway_lock_test_empty").unwrap();
        let cmd = LockCommand {
            plan: "p1".to_string(),
            path: Some(tmp_dir.path().join("non_existent")),
        };

        assert_eq!(cmd.run(), 1);
    }
}
