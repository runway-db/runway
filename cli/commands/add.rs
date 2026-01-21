mod change;
mod plan;

pub(crate) use change::AddChangeCommand;
pub(crate) use plan::AddPlanCommand;

use crate::commands::ExecutableCommand;
use clap::{Args, Subcommand};
use runway::build::Project;
use std::path::PathBuf;

/// Add a new change or plan to the project.
#[derive(Args)]
pub(crate) struct AddCommand {
    #[command(subcommand)]
    command: AddSubcommand,

    /// Path to the runway project.
    #[arg(short, long, global = true)]
    path: Option<PathBuf>,
}

#[derive(Subcommand)]
pub(crate) enum AddSubcommand {
    /// Add a new change to the project.
    Change(AddChangeCommand),
    /// Add a new plan to the project.
    Plan(AddPlanCommand),
}

impl ExecutableCommand for AddCommand {
    fn run(&self) -> u8 {
        let target_path = self.path.clone().unwrap_or_else(|| PathBuf::from("."));
        let _project = match Project::load(&target_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error loading project at {:?}: {e}", target_path);
                return 1;
            }
        };

        match &self.command {
            AddSubcommand::Change(cmd) => cmd.run(&target_path),
            AddSubcommand::Plan(cmd) => cmd.run(&target_path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempdir::TempDir;

    fn setup_test_project() -> TempDir {
        let tmp_dir = TempDir::new("runway_add_test").unwrap();
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("empty");

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
    fn test_add_change() {
        let tmp_dir = setup_test_project();
        let cmd = AddCommand {
            command: AddSubcommand::Change(AddChangeCommand {
                name: "new_change".to_string(),
            }),
            path: Some(tmp_dir.path().to_path_buf()),
        };

        assert_eq!(cmd.run(), 0);
        assert!(tmp_dir.path().join("changes/new_change").is_dir());
        assert!(
            tmp_dir
                .path()
                .join("changes/new_change/change.toml")
                .exists()
        );
        assert!(
            tmp_dir
                .path()
                .join("changes/new_change/deploy.sql")
                .exists()
        );
    }

    #[test]
    fn test_add_plan() {
        let tmp_dir = setup_test_project();
        let cmd = AddCommand {
            command: AddSubcommand::Plan(AddPlanCommand {
                name: "new_plan".to_string(),
            }),
            path: Some(tmp_dir.path().to_path_buf()),
        };

        assert_eq!(cmd.run(), 0);
        assert!(tmp_dir.path().join("plans/new_plan").is_dir());
        assert!(tmp_dir.path().join("plans/new_plan/plan.toml").exists());
    }

    #[test]
    fn test_add_change_already_exists() {
        let tmp_dir = setup_test_project();
        fs::create_dir_all(tmp_dir.path().join("changes/existing")).unwrap();

        let cmd = AddCommand {
            command: AddSubcommand::Change(AddChangeCommand {
                name: "existing".to_string(),
            }),
            path: Some(tmp_dir.path().to_path_buf()),
        };

        assert_eq!(cmd.run(), 1);
    }

    #[test]
    fn test_add_plan_already_exists() {
        let tmp_dir = setup_test_project();
        fs::create_dir_all(tmp_dir.path().join("plans/existing")).unwrap();

        let cmd = AddCommand {
            command: AddSubcommand::Plan(AddPlanCommand {
                name: "existing".to_string(),
            }),
            path: Some(tmp_dir.path().to_path_buf()),
        };

        assert_eq!(cmd.run(), 1);
    }
}
