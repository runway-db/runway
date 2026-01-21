mod change;
mod plan;

pub(crate) use change::ListChangeCommand;
pub(crate) use plan::ListPlanCommand;

use crate::commands::{ExecutableCommand, OutputFormat};
use clap::{Args, Subcommand};
use runway::build::Project;
use std::path::PathBuf;

/// List changes or plans in the project.
#[derive(Args)]
pub(crate) struct ListCommand {
    #[command(subcommand)]
    command: ListSubcommand,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::List, global = true)]
    format: OutputFormat,

    /// Path to the runway project.
    #[arg(short, long, global = true)]
    path: Option<PathBuf>,
}

#[derive(Subcommand)]
pub(crate) enum ListSubcommand {
    /// List all changes
    Change(ListChangeCommand),
    /// List all plans
    Plan(ListPlanCommand),
}

impl ExecutableCommand for ListCommand {
    fn run(&self) -> u8 {
        let target_path = self.path.clone().unwrap_or_else(|| PathBuf::from("."));
        let project = match Project::load(target_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error loading project: {e}");
                return 1;
            }
        };

        match &self.command {
            ListSubcommand::Change(cmd) => cmd.run(&project, &self.format),
            ListSubcommand::Plan(cmd) => cmd.run(&project, &self.format),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempdir::TempDir;

    fn setup_test_project() -> TempDir {
        let tmp_dir = TempDir::new("runway_list_test").unwrap();
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
    fn test_list_changes() {
        let tmp_dir = setup_test_project();
        let cmd = ListCommand {
            command: ListSubcommand::Change(ListChangeCommand {}),
            format: OutputFormat::List,
            path: Some(tmp_dir.path().to_path_buf()),
        };

        assert_eq!(cmd.run(), 0);
    }

    #[test]
    fn test_list_plans() {
        let tmp_dir = setup_test_project();
        let cmd = ListCommand {
            command: ListSubcommand::Plan(ListPlanCommand {}),
            format: OutputFormat::List,
            path: Some(tmp_dir.path().to_path_buf()),
        };

        assert_eq!(cmd.run(), 0);
    }

    #[test]
    fn test_list_non_existent_project() {
        let tmp_dir = TempDir::new("runway_list_test_empty").unwrap();
        let cmd = ListCommand {
            command: ListSubcommand::Change(ListChangeCommand {}),
            format: OutputFormat::List,
            path: Some(tmp_dir.path().join("non_existent")),
        };

        assert_eq!(cmd.run(), 1);
    }
}
