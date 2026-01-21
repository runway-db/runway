use runway::{DatabaseEngine, load_project, Package, MigrationSource};
use std::path::Path;
use std::error::Error;

pub enum DynamicSource {
    Project(runway::build::Project),
    Package(Package),
}

impl MigrationSource for DynamicSource {
    fn engine_metadata(&mut self, engine: &DatabaseEngine) -> Result<runway::package::metadata::EngineMetadata, Box<dyn Error>> {
        match self {
            Self::Project(p) => p.engine_metadata(engine),
            Self::Package(p) => Ok(p.engine_metadata(engine)?),
        }
    }

    fn change_metadata(&mut self, engine: &DatabaseEngine, change_name: &str) -> Result<runway::package::metadata::ChangeMetadata, Box<dyn Error>> {
        match self {
            Self::Project(p) => p.change_metadata(engine, change_name),
            Self::Package(p) => Ok(p.change_metadata(engine, change_name)?),
        }
    }

    fn deploy_script(&mut self, engine: &DatabaseEngine, change_name: &str) -> Result<String, Box<dyn Error>> {
        match self {
            Self::Project(p) => p.deploy_script(engine, change_name),
            Self::Package(p) => Ok(p.deploy_script(engine, change_name)?),
        }
    }

    fn verify_script(&mut self, engine: &DatabaseEngine, change_name: &str) -> Result<Option<String>, Box<dyn Error>> {
        match self {
            Self::Project(p) => p.verify_script(engine, change_name),
            Self::Package(p) => Ok(p.verify_script(engine, change_name)?),
        }
    }

    fn revert_script(&mut self, engine: &DatabaseEngine, change_name: &str) -> Result<Option<String>, Box<dyn Error>> {
        match self {
            Self::Project(p) => p.revert_script(engine, change_name),
            Self::Package(p) => Ok(p.revert_script(engine, change_name)?),
        }
    }
}

pub fn load_source(path: &Path) -> Result<DynamicSource, Box<dyn Error>> {
    // Check if it's a zip file
    if let Some(ext) = path.extension() {
        if ext == "zip" {
            return Ok(DynamicSource::Package(Package::load(path)?));
        }
    }

    // Check if it's a directory with runway.toml
    if path.is_dir() {
        if path.join("runway.toml").exists() {
            return Ok(DynamicSource::Project(load_project(path)?));
        }
    } else if path.exists() {
         // It might be a file that IS the package but doesn't have .zip extension? 
         // Unlikely for now, let's stick to .zip or directory.
    }

    Err(format!("Could not detect migration source at {:?}", path).into())
}
