pub(crate) mod graph;

use super::Change;
use super::DatabaseEngine;
use super::error::{BuildError, ChangeError, PlanError};
use super::plan::{Plan, PlanLock};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

fn discover_changes(path: impl AsRef<Path>) -> Result<Vec<DirEntry>, BuildError> {
    let path = path.as_ref();
    let dirs = fs::read_dir(&path)?
        .into_iter()
        .map(|item| -> Result<DirEntry, BuildError> { item.map_err(|e| e.into()) })
        .filter(|item| match item {
            Ok(entry) if entry.file_name().to_string_lossy().starts_with(".") => {
                log::debug!("Skipping hidden directory {:?}", entry.path());
                false
            }
            _ => true,
        })
        .filter_map(|item| -> Option<Result<DirEntry, BuildError>> {
            match item {
                Ok(entry) => match entry.file_type() {
                    Ok(file_type) if file_type.is_dir() => Some(Ok(entry)),
                    Ok(_) => None,
                    Err(e) => Some(Err(e.into())),
                },
                _ => Some(item),
            }
        })
        .filter_map(|item| match item {
            Ok(entry) => match entry.path().join("change.toml").try_exists() {
                Ok(true) => {
                    log::debug!("Discovered change at directory {:?}", entry.path());
                    Some(Ok(entry))
                }
                Ok(false) => {
                    log::debug!("Skipped directory {:?} missing change.toml", entry.path());
                    None
                }
                Err(e) => {
                    log::error!(
                        "Failed to check for change.toml in {:?}: {}",
                        entry.path(),
                        e
                    );
                    Some(Err(e.into()))
                }
            },
            Err(e) => Some(Err(e)),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(dirs)
}

fn discover_plans(path: impl AsRef<Path>) -> Result<Vec<DirEntry>, BuildError> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(vec![]);
    }
    let dirs = fs::read_dir(&path)?
        .into_iter()
        .map(|item| -> Result<DirEntry, BuildError> { item.map_err(|e| e.into()) })
        .filter(|item| match item {
            Ok(entry) if entry.file_name().to_string_lossy().starts_with(".") => {
                log::debug!("Skipping hidden directory {:?}", entry.path());
                false
            }
            _ => true,
        })
        .filter_map(|item| -> Option<Result<DirEntry, BuildError>> {
            match item {
                Ok(entry) => match entry.file_type() {
                    Ok(file_type) if file_type.is_dir() => Some(Ok(entry)),
                    Ok(_) => None,
                    Err(e) => Some(Err(e.into())),
                },
                _ => Some(item),
            }
        })
        .filter_map(|item| match item {
            Ok(entry) => match entry.path().join("plan.toml").try_exists() {
                Ok(true) => {
                    log::debug!("Discovered plan at directory {:?}", entry.path());
                    Some(Ok(entry))
                }
                Ok(false) => {
                    log::debug!("Skipped directory {:?} missing plan.toml", entry.path());
                    None
                }
                Err(e) => {
                    log::error!("Failed to check for plan.toml in {:?}: {}", entry.path(), e);
                    Some(Err(e.into()))
                }
            },
            Err(e) => Some(Err(e)),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(dirs)
}

#[derive(Deserialize, Debug)]
pub(crate) struct ProjectConfig {
    #[serde(default)]
    engines: Vec<DatabaseEngine>,
}

impl ProjectConfig {
    pub(crate) fn load(path: impl AsRef<Path>) -> Result<Self, BuildError> {
        let descriptor_path = path.as_ref().join("runway.toml");
        let bytes = fs::read(&descriptor_path).map_err(|e| BuildError::Io {
            path: descriptor_path.clone(),
            source: e,
        })?;
        let descriptor_data = String::from_utf8(bytes).map_err(|e| BuildError::Config {
            path: descriptor_path.clone(),
            source: e.into(),
        })?;

        toml::from_str::<Self>(&descriptor_data).map_err(|e| BuildError::Config {
            path: descriptor_path,
            source: e.into(),
        })
    }
}

#[derive(Debug)]
pub(crate) struct ProjectState {
    pub(crate) changes: RwLock<HashMap<String, Arc<super::change::ChangeInner>>>,
    pub(crate) plans: RwLock<HashMap<String, Arc<super::plan::PlanInner>>>,
}

pub struct Project {
    path: PathBuf,
    config: ProjectConfig,
    state: Arc<ProjectState>,
    sequences: RwLock<HashMap<DatabaseEngine, Vec<graph::GraphNode>>>,
}

impl Project {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, BuildError> {
        let path = path.as_ref().to_path_buf();
        let change_path = path.join("changes");
        let plan_path = path.join("plans");
        log::debug!("Loading project from {:?}", path);
        let config = ProjectConfig::load(&path)?;

        let state = Arc::new(ProjectState {
            changes: RwLock::new(HashMap::new()),
            plans: RwLock::new(HashMap::new()),
        });

        {
            let mut changes_lock = state.changes.write().expect("Lock poisoned");
            for dir in discover_changes(&change_path)? {
                let change_inner =
                    Change::load(dir.path(), Arc::clone(&state)).map_err(|source| {
                        BuildError::Change {
                            name: dir.file_name().to_string_lossy().to_string(),
                            source,
                        }
                    })?;
                changes_lock.insert(change_inner.name.clone(), change_inner);
            }
        }

        {
            let mut plans_lock = state.plans.write().expect("Lock poisoned");
            for dir in discover_plans(&plan_path)? {
                let plan_inner = Plan::load(dir.path(), Arc::clone(&state)).map_err(|source| {
                    BuildError::Plan {
                        name: dir.file_name().to_string_lossy().to_string(),
                        source,
                    }
                })?;
                plans_lock.insert(plan_inner.name.clone(), plan_inner);
            }
        }

        let sequences = RwLock::new(HashMap::new());

        Ok(Self {
            path,
            config,
            state,
            sequences,
        })
    }

    pub fn all_changes(&self) -> Vec<Change> {
        self.state
            .changes
            .read()
            .expect("Lock poisoned")
            .values()
            .map(|inner| Change::new(Arc::clone(inner), Arc::clone(&self.state)))
            .collect()
    }

    pub fn enabled_engines(&self) -> &[DatabaseEngine] {
        &self.config.engines
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn changes_for_engine(
        &self,
        engine: &DatabaseEngine,
    ) -> Result<Vec<graph::GraphNode>, BuildError> {
        if let Some(nodes) = self.sequences.read().expect("Lock poisoned").get(engine) {
            return Ok(nodes.clone());
        }

        let changes = self.state.changes.read().expect("Lock poisoned");
        let plans = self.state.plans.read().expect("Lock poisoned");

        // Verify integrity of all plans for this engine
        for plan_inner in plans.values() {
            let plan = Plan::new(Arc::clone(plan_inner), Arc::clone(&self.state));
            plan.verify_integrity(engine)?;
        }

        let nodes = graph::calculate_change_graph(&changes, &plans, &self.state, engine)?;

        self.sequences
            .write()
            .expect("Lock poisoned")
            .insert(engine.clone(), nodes.clone());

        Ok(nodes)
    }

    pub fn plans(&self) -> Vec<Plan> {
        self.state
            .plans
            .read()
            .expect("Lock poisoned")
            .values()
            .map(|inner| Plan::new(Arc::clone(inner), Arc::clone(&self.state)))
            .collect()
    }

    pub fn get_plan(&self, name: &str) -> Option<Plan> {
        self.state
            .plans
            .read()
            .expect("Lock poisoned")
            .get(name)
            .map(|inner| Plan::new(Arc::clone(inner), Arc::clone(&self.state)))
    }

    pub fn get_plan_mut(&self, name: &str) -> Option<Plan> {
        self.get_plan(name)
    }

    pub fn lock_plan(&mut self, plan_name: &str) -> Result<(), BuildError> {
        let engines = self.config.engines.clone();
        if engines.is_empty() {
            return Err(BuildError::Other(
                "No engines configured for project".to_string(),
            ));
        }

        let (plan_targets, plan_parent, plan_path) = {
            let plan = self
                .get_plan(plan_name)
                .ok_or_else(|| BuildError::Other(format!("Plan '{}' not found", plan_name)))?;
            (
                plan.targets().to_vec(),
                plan.parent().cloned(),
                self.path.join("plans").join(plan_name),
            )
        };

        let mut engine_locks = BTreeMap::new();

        for engine in &engines {
            let mut dependencies = BTreeMap::new();

            for target in &plan_targets {
                let changes = self.state.changes.read().expect("Lock poisoned");
                let change_inner = changes.get(target).ok_or_else(|| {
                    BuildError::Other(format!("Target change '{}' not found in project", target))
                })?;
                let change = Change::new(Arc::clone(change_inner), Arc::clone(&self.state));

                let hash = change.hash(engine).map_err(|source| BuildError::Change {
                    name: target.clone(),
                    source: ChangeError::Unexpected(source.to_string()),
                })?;

                dependencies.insert(target.clone(), hash);
            }

            if let Some(parent_name) = &plan_parent {
                let plan = self.get_plan(parent_name).ok_or_else(|| {
                    BuildError::Other(format!("Parent plan '{}' not found", parent_name))
                })?;

                let hash = plan.hash(engine).map_err(|source| BuildError::Plan {
                    name: parent_name.clone(),
                    source: PlanError::Unexpected(source.to_string()),
                })?;

                dependencies.insert(format!("@{}", parent_name), hash);
            }

            engine_locks.insert(engine.clone(), dependencies);
        }

        let lock = PlanLock {
            name: plan_name.to_string(),
            engines: engine_locks,
        };

        lock.write(&plan_path).map_err(|source| BuildError::Plan {
            name: plan_name.to_string(),
            source: PlanError::Lock { source },
        })?;

        if let Some(plan) = self.get_plan_mut(plan_name) {
            plan.set_lock(lock);
        }

        Ok(())
    }
}

impl crate::migrator::MigrationSource for Project {
    fn engine_metadata(
        &mut self,
        engine: &DatabaseEngine,
    ) -> Result<crate::package::metadata::EngineMetadata, Box<dyn std::error::Error>> {
        let nodes = self.changes_for_engine(engine)?;
        let sequence = nodes
            .into_iter()
            .filter_map(|node| match node {
                crate::build::project::graph::GraphNode::Change(c) => Some(c.name().clone()),
                crate::build::project::graph::GraphNode::Plan(p) => Some(format!("@{}", p.name())),
                crate::build::project::graph::GraphNode::Root => None,
            })
            .collect();

        Ok(crate::package::metadata::EngineMetadata::new(sequence))
    }

    fn change_metadata(
        &mut self,
        engine: &DatabaseEngine,
        change_name: &str,
    ) -> Result<crate::package::metadata::ChangeMetadata, Box<dyn std::error::Error>> {
        let changes = self.state.changes.read().expect("Lock poisoned");
        let change_inner = changes
            .get(change_name)
            .ok_or_else(|| format!("Change {} not found", change_name))?;
        let change = Change::new(Arc::clone(change_inner), Arc::clone(&self.state));

        Ok(crate::package::metadata::ChangeMetadata::new(
            &change, engine,
        )?)
    }

    fn deploy_script(
        &mut self,
        engine: &DatabaseEngine,
        change_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let changes = self.state.changes.read().expect("Lock poisoned");
        let change_inner = changes
            .get(change_name)
            .ok_or_else(|| format!("Change {} not found", change_name))?;
        let change = Change::new(Arc::clone(change_inner), Arc::clone(&self.state));
        let scripts = change.scripts_for_engine(engine)?;

        Ok(scripts.deploy().clone())
    }

    fn verify_script(
        &mut self,
        engine: &DatabaseEngine,
        change_name: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let changes = self.state.changes.read().expect("Lock poisoned");
        let change_inner = changes
            .get(change_name)
            .ok_or_else(|| format!("Change {} not found", change_name))?;
        let change = Change::new(Arc::clone(change_inner), Arc::clone(&self.state));
        let scripts = change.scripts_for_engine(engine)?;

        Ok(scripts.verify().cloned())
    }

    fn revert_script(
        &mut self,
        engine: &DatabaseEngine,
        change_name: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let changes = self.state.changes.read().expect("Lock poisoned");
        let change_inner = changes
            .get(change_name)
            .ok_or_else(|| format!("Change {} not found", change_name))?;
        let change = Change::new(Arc::clone(change_inner), Arc::clone(&self.state));
        let scripts = change.scripts_for_engine(engine)?;

        Ok(scripts.revert().cloned())
    }
}
