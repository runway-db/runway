use crate::package::{DirectoryWriterBackend, PackageWriter, PackageWriterBackend, ZipWriterBackend};
use crate::package::metadata::{EngineMetadata, PackageMetadata};
use std::fs::OpenOptions;
use std::path::Path;

pub(crate) mod change;
pub(crate) mod error;
pub(crate) mod plan;
pub(crate) mod project;

use crate::DatabaseEngine;
pub use change::{Change, ScriptCollection};
pub use error::{BuildError, ChangeError, DescriptorError, PlanError, ScriptError};
pub use plan::{Plan, PlanLock};
pub use project::Project;
pub use project::graph::{ChangeGraphError, GraphNode};

pub fn load_project(path: impl AsRef<Path>) -> Result<Project, BuildError> {
    Project::load(path)
}

fn write_package_contents<B: PackageWriterBackend>(
    writer: &mut PackageWriter<B>,
    project: &Project,
) -> Result<(), BuildError> {
    let engines = project.enabled_engines().to_vec();
    writer
        .add_package_metadata(&PackageMetadata::new(engines.clone()))
        .map_err(|e| BuildError::Package(e.to_string()))?;

    for engine in &engines {
        writer
            .add_engine(engine)
            .map_err(|e| BuildError::Package(e.to_string()))?;

        // Verify integrity of all plans for this engine
        for plan in project.plans() {
            plan.verify_integrity(engine)?;
        }

        let nodes = project.changes_for_engine(engine)?;
        for node in &nodes {
            writer
                .add_node(engine, node)
                .map_err(|e| BuildError::Package(e.to_string()))?;
        }

        let sequence = nodes
            .into_iter()
            .filter_map(|node| match node {
                GraphNode::Change(c) => Some(c.name().clone()),
                GraphNode::Plan(p) => Some(format!("@{}", p.name())),
                GraphNode::Root => None,
            })
            .collect();
        writer
            .add_engine_metadata(engine, &EngineMetadata::new(sequence))
            .map_err(|e| BuildError::Package(e.to_string()))?;
    }
    Ok(())
}

pub fn package_migrations(
    path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    exploded: bool,
) -> Result<(), BuildError> {
    let project = Project::load(path)?;

    if exploded {
        let backend = DirectoryWriterBackend::new(output_path.as_ref().to_path_buf())
            .map_err(|e| BuildError::Package(e.to_string()))?;
        let mut writer = PackageWriter::new(backend)
            .map_err(|e| BuildError::Package(e.to_string()))?;
        write_package_contents(&mut writer, &project)?;
        writer
            .finalize()
            .map_err(|e| BuildError::Package(e.to_string()))?;
    } else {
        let file = OpenOptions::new()
            .truncate(true)
            .write(true)
            .create(true)
            .open(output_path)
            .map_err(BuildError::from)?;

        let backend = ZipWriterBackend::new(file);
        let mut writer = PackageWriter::new(backend)
            .map_err(|e| BuildError::Package(e.to_string()))?;
        write_package_contents(&mut writer, &project)?;
        writer
            .finalize()
            .map_err(|e| BuildError::Package(e.to_string()))?;
    }

    Ok(())
}

#[cfg(feature = "build")]
pub fn package_project(path: impl AsRef<Path>) -> Result<(), BuildError> {
    package_named_project(path, "migrations")
}

#[cfg(feature = "build")]
pub fn package_named_project(path: impl AsRef<Path>, name: &str) -> Result<(), BuildError> {
    let out_dir =
        std::env::var("OUT_DIR").map_err(|_| BuildError::Package("OUT_DIR not set".to_string()))?;
    let output_path = Path::new(&out_dir).join(format!("{}.runway", name));
    package_migrations(path.as_ref(), &output_path, false)?;

    println!("cargo:rerun-if-changed={}", path.as_ref().display());
    Ok(())
}
