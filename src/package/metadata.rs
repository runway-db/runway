use crate::DatabaseEngine;
#[cfg(feature = "generation")]
use crate::build::{Change, plan::Plan};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PlanMetadata {
    name: String,
    hash: Option<String>,
}

#[cfg(feature = "generation")]
impl PlanMetadata {
    pub(crate) fn new(plan: &Plan, hash: Option<String>) -> Self {
        Self {
            name: plan.name().clone(),
            hash,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PackageMetadata {
    engines: Vec<DatabaseEngine>,
}

impl PackageMetadata {
    pub fn new(engines: Vec<DatabaseEngine>) -> Self {
        Self { engines }
    }

    pub fn engines(&self) -> &[DatabaseEngine] {
        &self.engines
    }
}

#[derive(Serialize, Deserialize)]
pub struct ChangeMetadata {
    name: String,
    hash: String,
}

impl ChangeMetadata {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }
}

#[cfg(feature = "generation")]
impl ChangeMetadata {
    pub(crate) fn new(
        change: &Change,
        engine: &DatabaseEngine,
    ) -> Result<Self, crate::build::ScriptError> {
        let name = change.name().clone();
        let hash = change.hash(engine)?;

        Ok(Self { name, hash })
    }
}

#[derive(Serialize, Deserialize)]
pub struct EngineMetadata {
    sequence: Vec<String>,
}

impl EngineMetadata {
    pub fn new(sequence: Vec<String>) -> Self {
        Self { sequence }
    }

    pub fn sequence(&self) -> &[String] {
        &self.sequence
    }
}
