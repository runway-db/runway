use crate::build::{DescriptorError, PlanError, ScriptError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub(crate) struct PlanDescriptor {
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub targets: Vec<String>,
}

impl PlanDescriptor {
    pub(crate) fn load(path: impl AsRef<Path>) -> Result<Self, DescriptorError> {
        let descriptor_path = path.as_ref().join("plan.toml");
        let bytes = fs::read(&descriptor_path)?;
        let descriptor_data = String::from_utf8(bytes)?;

        Ok(toml::from_str::<Self>(&descriptor_data)?)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct PlanLock {
    pub name: String,
    pub engines: BTreeMap<crate::DatabaseEngine, BTreeMap<String, String>>,
}

impl PlanLock {
    pub(crate) fn load(path: impl AsRef<Path>) -> Result<Self, DescriptorError> {
        let lock_path = path.as_ref().join("plan.lock");
        let bytes = fs::read(&lock_path)?;
        let lock_data = String::from_utf8(bytes)?;

        Ok(toml::from_str::<Self>(&lock_data)?)
    }

    pub(crate) fn write(&self, path: impl AsRef<Path>) -> Result<(), DescriptorError> {
        let lock_path = path.as_ref().join("plan.lock");
        let lock_data = toml::to_string_pretty(self)?;

        fs::write(&lock_path, lock_data)?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct PlanInner {
    pub(crate) name: String,
    pub(crate) descriptor: PlanDescriptor,
    pub(crate) lock: RwLock<Option<PlanLock>>,
    pub(crate) hashes: RwLock<HashMap<crate::DatabaseEngine, String>>,
}

#[derive(Debug)]
pub struct Plan {
    inner: Arc<PlanInner>,
    state: Arc<super::project::ProjectState>,
}

impl Clone for Plan {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            state: Arc::clone(&self.state),
        }
    }
}

impl PartialEq for Plan {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for Plan {}

impl Plan {
    pub(crate) fn load(
        path: impl AsRef<Path>,
        _state: Arc<super::project::ProjectState>,
    ) -> Result<Arc<PlanInner>, PlanError> {
        let path = path.as_ref().to_path_buf();
        let descriptor =
            PlanDescriptor::load(&path).map_err(|source| PlanError::Descriptor { source })?;

        let name = path
            .file_name()
            .ok_or_else(|| {
                PlanError::Unexpected(format!("could not determine file name for path {:?}", path))
            })?
            .to_string_lossy()
            .into();

        let lock = if path.join("plan.lock").exists() {
            Some(PlanLock::load(&path).map_err(|source| PlanError::Lock { source })?)
        } else {
            None
        };

        Ok(Arc::new(PlanInner {
            name,
            descriptor,
            lock: RwLock::new(lock),
            hashes: RwLock::new(HashMap::new()),
        }))
    }

    pub(crate) fn new(inner: Arc<PlanInner>, state: Arc<super::project::ProjectState>) -> Self {
        Self { inner, state }
    }

    pub fn name(&self) -> &String {
        &self.inner.name
    }

    pub fn parent(&self) -> Option<&String> {
        self.inner.descriptor.parent.as_ref()
    }

    pub fn targets(&self) -> &[String] {
        &self.inner.descriptor.targets
    }

    pub fn lock(&self) -> Option<PlanLock> {
        self.inner.lock.read().expect("Lock poisoned").clone()
    }

    pub(crate) fn set_lock(&self, lock: PlanLock) {
        *self.inner.lock.write().expect("Lock poisoned") = Some(lock);
    }

    pub fn verify_integrity(
        &self,
        engine: &crate::DatabaseEngine,
    ) -> Result<(), crate::build::BuildError> {
        let lock = match self.lock() {
            Some(l) => l,
            None => return Ok(()), // No lock, nothing to verify
        };

        let engine_lock =
            lock.engines
                .get(engine)
                .ok_or_else(|| crate::build::BuildError::Plan {
                    name: self.name().clone(),
                    source: PlanError::Unexpected(format!("No lock data for engine {}", engine)),
                })?;

        let state = &self.state;

        // Verify parent plan
        if let Some(parent_name) = self.parent() {
            let plans = state.plans.read().expect("Lock poisoned");
            let parent_inner =
                plans
                    .get(parent_name)
                    .ok_or_else(|| crate::build::BuildError::Plan {
                        name: self.name().clone(),
                        source: PlanError::Unexpected(format!(
                            "Parent plan '{}' not found",
                            parent_name
                        )),
                    })?;
            let parent_plan = Plan::new(Arc::clone(parent_inner), Arc::clone(state));

            let current_hash = parent_plan
                .hash(engine)
                .map_err(|e| crate::build::BuildError::Script(e))?;
            let locked_hash = engine_lock
                .get(&format!("@{}", parent_name))
                .ok_or_else(|| crate::build::BuildError::Plan {
                    name: self.name().clone(),
                    source: PlanError::Unexpected(format!(
                        "Parent plan '@{}' missing from lock",
                        parent_name
                    )),
                })?;

            if current_hash != *locked_hash {
                return Err(crate::build::BuildError::Plan {
                    name: self.name().clone(),
                    source: PlanError::LockMismatch {
                        target: format!("@{}", parent_name),
                        engine: engine.clone(),
                        expected: locked_hash.clone(),
                        found: current_hash,
                    },
                });
            }

            // Recursively verify parent plan's integrity
            parent_plan.verify_integrity(engine)?;
        }

        // Verify Changes
        for target in self.targets() {
            let changes = state.changes.read().expect("Lock poisoned");
            let change_inner =
                changes
                    .get(target)
                    .ok_or_else(|| crate::build::BuildError::Plan {
                        name: self.name().clone(),
                        source: PlanError::Unexpected(format!(
                            "Target change '{}' not found",
                            target
                        )),
                    })?;
            let change = super::Change::new(Arc::clone(change_inner), Arc::clone(state));

            let current_hash = change
                .hash(engine)
                .map_err(|e| crate::build::BuildError::Script(e))?;
            let locked_hash =
                engine_lock
                    .get(target)
                    .ok_or_else(|| crate::build::BuildError::Plan {
                        name: self.name().clone(),
                        source: PlanError::Unexpected(format!(
                            "Target change '{}' missing from lock",
                            target
                        )),
                    })?;

            if current_hash != *locked_hash {
                return Err(crate::build::BuildError::Plan {
                    name: self.name().clone(),
                    source: PlanError::LockMismatch {
                        target: target.clone(),
                        engine: engine.clone(),
                        expected: locked_hash.clone(),
                        found: current_hash,
                    },
                });
            }
        }

        Ok(())
    }

    pub fn hash(&self, engine: &crate::DatabaseEngine) -> Result<String, ScriptError> {
        use sha2::{Digest, Sha256};

        if let Some(lock) = self.lock() {
            if lock.engines.contains_key(engine) {
                // If we're locked for this engine, we should probably check if all targets are present
                // But for now, let's just use the locked hash if it exists.
                // Wait, Plan::hash is the hash of the PLAN itself, which depends on its targets' hashes.
                // In the lockfile, we store the hashes of the dependencies.

                // If we are locked, we don't calculate a new hash, we return the one that represents
                // the plan's state at the time of locking.
                // Actually, the Plan doesn't have its own hash in the lockfile?
                // Ah, Project::lock_plan stores dependencies.
            }
        }

        if let Some(hash) = self.inner.hashes.read().expect("Lock poisoned").get(engine) {
            return Ok(hash.clone());
        }

        let mut hasher = Sha256::new();
        hasher.update(self.name());

        let state = &self.state;

        if let Some(parent_name) = self.parent() {
            let plans = state.plans.read().expect("Lock poisoned");
            let parent_inner = plans
                .get(parent_name)
                .ok_or_else(|| ScriptError::ReadFailed {
                    path: PathBuf::from(parent_name),
                    source: std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Parent plan '{}' not found", parent_name),
                    ),
                })?;
            let parent_plan = Plan::new(Arc::clone(parent_inner), Arc::clone(state));

            let hash = if let Some(lock) = self.lock() {
                lock.engines
                    .get(engine)
                    .and_then(|deps| deps.get(&format!("@{}", parent_name)).cloned())
                    .map(Ok)
                    .unwrap_or_else(|| parent_plan.hash(engine))?
            } else {
                parent_plan.hash(engine)?
            };
            hasher.update(hash);
        }

        for target in self.targets() {
            let changes = state.changes.read().expect("Lock poisoned");
            let change_inner = changes.get(target).ok_or_else(|| ScriptError::ReadFailed {
                path: PathBuf::from(target),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Target change '{}' not found", target),
                ),
            })?;
            let change = super::Change::new(Arc::clone(change_inner), Arc::clone(state));

            let hash = if let Some(lock) = self.lock() {
                lock.engines
                    .get(engine)
                    .and_then(|deps| deps.get(target).cloned())
                    .map(Ok)
                    .unwrap_or_else(|| change.hash(engine))?
            } else {
                change.hash(engine)?
            };
            hasher.update(hash);
        }

        let hash = format!("{:x}", hasher.finalize());
        self.inner
            .hashes
            .write()
            .expect("Lock poisoned")
            .insert(engine.clone(), hash.clone());
        Ok(hash)
    }
}
