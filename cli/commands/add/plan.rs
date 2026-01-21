use clap::Args;
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub(crate) struct AddPlanCommand {
    /// Name of the plan.
    pub name: String,
}

impl AddPlanCommand {
    pub fn run(&self, target_path: &PathBuf) -> u8 {
        let name = &self.name;
        let plan_dir = target_path.join("plans").join(name);
        if plan_dir.exists() {
            eprintln!("Error: Plan '{}' already exists at {:?}", name, plan_dir);
            return 1;
        }

        if let Err(e) = fs::create_dir_all(&plan_dir) {
            eprintln!("Error creating plan directory {:?}: {}", plan_dir, e);
            return 1;
        }

        if let Err(e) = fs::write(plan_dir.join("plan.toml"), "targets = []\n") {
            eprintln!("Error creating plan.toml: {}", e);
            return 1;
        }

        println!("Added new plan '{}' in {:?}", name, plan_dir);
        0
    }
}
