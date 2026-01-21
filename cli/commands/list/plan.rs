use crate::commands::OutputFormat;
use clap::Args;
use runway::build::Project;
use serde::Serialize;
use tabled::Tabled;

#[derive(Args)]
pub(crate) struct ListPlanCommand {}

#[derive(Tabled, Serialize)]
struct PlanRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Parent")]
    parent: String,
}

impl ListPlanCommand {
    pub fn run(&self, project: &Project, format: &OutputFormat) -> u8 {
        let mut plans = project.plans();
        plans.sort_by(|a, b| a.name().cmp(b.name()));

        match format {
            OutputFormat::List => {
                for plan in plans {
                    println!("{}", plan.name());
                }
            }
            OutputFormat::Json => {
                let rows: Vec<_> = plans
                    .iter()
                    .map(|p| PlanRow {
                        name: p.name().clone(),
                        parent: p.parent().cloned().unwrap_or_default(),
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&rows).unwrap());
            }
            OutputFormat::Table => {
                let rows: Vec<_> = plans
                    .iter()
                    .map(|p| PlanRow {
                        name: p.name().clone(),
                        parent: p.parent().cloned().unwrap_or_default(),
                    })
                    .collect();
                println!("{}", tabled::Table::new(rows).to_string());
            }
        }
        0
    }
}
