use crate::commands::OutputFormat;
use clap::Args;
use runway::build::Project;
use serde::Serialize;
use tabled::Tabled;

#[derive(Args)]
pub(crate) struct ListChangeCommand {}

#[derive(Tabled, Serialize)]
struct ChangeRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Description")]
    description: String,
}

impl ListChangeCommand {
    pub fn run(&self, project: &Project, format: &OutputFormat) -> u8 {
        let mut changes = project.all_changes();
        changes.sort_by(|a, b| a.name().cmp(b.name()));

        match format {
            OutputFormat::List => {
                for change in changes {
                    println!("{}", change.name());
                }
            }
            OutputFormat::Json => {
                let rows: Vec<_> = changes
                    .iter()
                    .map(|c| ChangeRow {
                        name: c.name().clone(),
                        description: c.description().clone(),
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&rows).unwrap());
            }
            OutputFormat::Table => {
                let rows: Vec<_> = changes
                    .iter()
                    .map(|c| ChangeRow {
                        name: c.name().clone(),
                        description: c.description().clone(),
                    })
                    .collect();
                println!("{}", tabled::Table::new(rows).to_string());
            }
        }
        0
    }
}
