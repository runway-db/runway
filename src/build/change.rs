mod scripts;

use crate::DatabaseEngine;
use crate::build::{ChangeError, DescriptorError, ScriptError};
pub use scripts::ScriptCollection;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
struct EngineConfig {
    #[serde(default = "yes")]
    enabled: bool,
    #[serde(default)]
    requires: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq)]
pub(crate) struct ChangeDescriptor {
    #[serde(default)]
    description: String,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    reworks: Option<String>,
    #[serde(default)]
    engine: BTreeMap<DatabaseEngine, EngineConfig>,
}

impl ChangeDescriptor {
    pub(crate) fn load(path: impl AsRef<Path>) -> Result<Self, DescriptorError> {
        let descriptor_path = path.as_ref().join("change.toml");
        let bytes = fs::read(&descriptor_path)?;
        let descriptor_data = String::from_utf8(bytes)?;

        Ok(toml::from_str::<Self>(&descriptor_data)?)
    }
}

#[derive(Debug)]
pub(crate) struct ChangeInner {
    pub(crate) name: String,
    pub(crate) descriptor: ChangeDescriptor,
    pub(crate) path: PathBuf,
    pub(crate) hashes: RwLock<HashMap<DatabaseEngine, String>>,
}

#[derive(Clone, Debug)]
pub struct Change {
    inner: Arc<ChangeInner>,
    #[allow(dead_code)]
    state: Arc<super::project::ProjectState>,
}

impl PartialEq for Change {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for Change {}

impl std::hash::Hash for Change {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.name.hash(state);
    }
}

impl Change {
    pub(crate) fn load(
        path: impl AsRef<Path>,
        _state: Arc<super::project::ProjectState>,
    ) -> Result<Arc<ChangeInner>, ChangeError> {
        let path = path.as_ref().to_path_buf();
        let descriptor =
            ChangeDescriptor::load(&path).map_err(|source| ChangeError::Descriptor { source })?;

        let name = path
            .file_name()
            .ok_or_else(|| {
                ChangeError::Unexpected(format!(
                    "could not determine file name for path {:?}",
                    path
                ))
            })?
            .to_string_lossy()
            .into();

        Ok(Arc::new(ChangeInner {
            name,
            descriptor,
            path,
            hashes: RwLock::new(HashMap::new()),
        }))
    }

    pub(crate) fn new(inner: Arc<ChangeInner>, state: Arc<super::project::ProjectState>) -> Self {
        Self { inner, state }
    }

    pub fn path(&self) -> &PathBuf {
        &self.inner.path
    }

    pub fn requires(&self) -> Vec<String> {
        let mut result = self.inner.descriptor.requires.clone();
        result.sort();
        result
    }

    pub fn reworks(&self) -> Option<&String> {
        self.inner.descriptor.reworks.as_ref()
    }

    pub fn enabled_for_engine(&self, engine: &DatabaseEngine) -> bool {
        self.inner
            .descriptor
            .engine
            .get(engine)
            .map_or(true, |x| x.enabled)
    }

    pub fn requires_for_engine(&self, engine: &DatabaseEngine) -> Vec<String> {
        let shared_requirements = self.inner.descriptor.requires.clone().into_iter();
        let engine_requirements = self
            .inner
            .descriptor
            .engine
            .get(&engine)
            .map(|x| x.requires.clone())
            .unwrap_or_else(|| vec![])
            .into_iter();

        let mut result: HashSet<String> =
            HashSet::with_capacity(shared_requirements.len() + engine_requirements.len());
        result.extend(shared_requirements);
        result.extend(engine_requirements);
        let mut result = result.into_iter().collect::<Vec<_>>();
        result.sort();
        result
    }

    pub fn scripts_for_engine(
        &self,
        engine: &DatabaseEngine,
    ) -> Result<ScriptCollection, ScriptError> {
        ScriptCollection::load_for_engine(&self.inner.path, engine)
    }

    pub fn name(&self) -> &String {
        &self.inner.name
    }

    pub fn description(&self) -> &String {
        &self.inner.descriptor.description
    }

    pub fn hash(&self, engine: &DatabaseEngine) -> Result<String, ScriptError> {
        use sha2::{Digest, Sha256};

        if let Some(hash) = self.inner.hashes.read().expect("Lock poisoned").get(engine) {
            return Ok(hash.clone());
        }

        let scripts = self.scripts_for_engine(engine)?;
        let requirements = self.requires_for_engine(engine);

        let hash = {
            let mut hasher = Sha256::new();
            hasher.update(self.name());
            for requirement in requirements {
                hasher.update(requirement);
            }
            if let Some(reworks) = self.reworks() {
                hasher.update(reworks);
            }
            hasher.update(scripts.deploy());
            hasher.update(scripts.revert().map(|x: &String| x.as_str()).unwrap_or(""));
            hasher.update(scripts.verify().map(|x: &String| x.as_str()).unwrap_or(""));
            format!("{:x}", hasher.finalize())
        };

        self.inner
            .hashes
            .write()
            .expect("Lock poisoned")
            .insert(engine.clone(), hash.clone());

        Ok(hash)
    }
}
