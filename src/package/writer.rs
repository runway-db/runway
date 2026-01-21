use crate::DatabaseEngine;
use crate::build::plan::Plan;
use crate::build::project::graph::GraphNode;
use crate::build::{Change, ScriptError};
use crate::package::metadata::{ChangeMetadata, EngineMetadata, PackageMetadata, PlanMetadata};
use serde::Serialize;
use std::io::{Seek, Write};
use std::path::PathBuf;
use wherror::Error;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[derive(Debug, Error)]
pub(crate) enum PackageWriterError {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Serialization(#[from] serde_json::error::Error),
    #[error("failed to load script: {source:?}")]
    ScriptLoadFailed {
        #[source]
        source: ScriptError,
    },
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
}

impl From<ScriptError> for PackageWriterError {
    fn from(value: ScriptError) -> Self {
        match value {
            ScriptError::ReadFailed { source, .. } => Self::IO(source),
            _ => Self::ScriptLoadFailed { source: value },
        }
    }
}

pub(crate) trait PackageWriterBackend {
    fn write_file(&mut self, path: &str, contents: &[u8]) -> Result<(), PackageWriterError>;
    fn add_directory(&mut self, path: &str) -> Result<(), PackageWriterError>;
    fn finish(self) -> Result<(), PackageWriterError>;

    fn write_json<T: Serialize>(&mut self, path: &str, data: &T) -> Result<(), PackageWriterError> {
        let contents = serde_json::to_vec_pretty(data)?;
        self.write_file(path, &contents)
    }
}

pub(crate) struct ZipWriterBackend<W: Write + Seek> {
    writer: ZipWriter<W>,
}

impl<W: Write + Seek> ZipWriterBackend<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: ZipWriter::new(writer),
        }
    }
}

impl<W: Write + Seek> PackageWriterBackend for ZipWriterBackend<W> {
    fn write_file(&mut self, path: &str, contents: &[u8]) -> Result<(), PackageWriterError> {
        self.writer.start_file(path, SimpleFileOptions::DEFAULT)?;
        self.writer.write_all(contents)?;
        Ok(())
    }

    fn add_directory(&mut self, path: &str) -> Result<(), PackageWriterError> {
        self.writer.add_directory(path, SimpleFileOptions::DEFAULT)?;
        Ok(())
    }

    fn finish(self) -> Result<(), PackageWriterError> {
        self.writer.finish()?;
        Ok(())
    }
}

pub(crate) struct DirectoryWriterBackend {
    root: PathBuf,
}

impl DirectoryWriterBackend {
    pub fn new(root: PathBuf) -> Result<Self, PackageWriterError> {
        if !root.exists() {
            std::fs::create_dir_all(&root)?;
        }
        Ok(Self { root })
    }
}

impl PackageWriterBackend for DirectoryWriterBackend {
    fn write_file(&mut self, path: &str, contents: &[u8]) -> Result<(), PackageWriterError> {
        let full_path = self.root.join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(full_path, contents)?;
        Ok(())
    }

    fn add_directory(&mut self, path: &str) -> Result<(), PackageWriterError> {
        let full_path = self.root.join(path);
        std::fs::create_dir_all(full_path)?;
        Ok(())
    }

    fn finish(self) -> Result<(), PackageWriterError> {
        Ok(())
    }
}

pub(crate) struct PackageWriter<B: PackageWriterBackend> {
    backend: B,
}

impl<B: PackageWriterBackend> PackageWriter<B> {
    pub(crate) fn new(mut backend: B) -> Result<Self, PackageWriterError> {
        backend.add_directory("engines")?;

        Ok(Self { backend })
    }

    pub(crate) fn add_package_metadata(
        &mut self,
        metadata: &PackageMetadata,
    ) -> Result<(), PackageWriterError> {
        self.backend.write_json("package.json", metadata)
    }

    pub(crate) fn add_change(
        &mut self,
        engine: &DatabaseEngine,
        change: &Change,
    ) -> Result<(), PackageWriterError> {
        let base_path = format!("engines/{}/{}", engine.identifier(), change.name());
        let scripts = change.scripts_for_engine(engine)?;
        {
            let metadata = ChangeMetadata::new(change, engine)?;
            self.backend.write_json(&format!("{}/metadata.json", base_path), &metadata)?;
        }

        self.backend.write_file(
            &format!("{}/deploy.sql", base_path),
            scripts.deploy().as_bytes(),
        )?;

        match scripts.revert() {
            Some(revert) => {
                self.backend.write_file(&format!("{}/revert.sql", base_path), revert.as_bytes())?
            }
            None => (),
        }

        match scripts.verify() {
            Some(verify) => {
                self.backend.write_file(&format!("{}/verify.sql", base_path), verify.as_bytes())
            }
            None => Ok(()),
        }
    }

    pub(crate) fn add_engine(&mut self, engine: &DatabaseEngine) -> Result<(), PackageWriterError> {
        log::trace!("Adding engine: {:?} to package", engine);
        self.backend.add_directory(&format!("engines/{}", engine.identifier()))
    }

    pub(crate) fn add_engine_metadata(
        &mut self,
        engine: &DatabaseEngine,
        metadata: &EngineMetadata,
    ) -> Result<(), PackageWriterError> {
        self.backend.write_json(
            &format!("engines/{}/metadata.json", engine.identifier()),
            metadata,
        )
    }

    pub(crate) fn add_plan_to_engine(
        &mut self,
        engine: &DatabaseEngine,
        plan: &Plan,
    ) -> Result<(), PackageWriterError> {
        let base_path = format!("engines/{}/@{}", engine.identifier(), plan.name());
        let hash = Some(plan.hash(engine)?);
        let metadata = PlanMetadata::new(plan, hash);
        self.backend.write_json(&format!("{}/metadata.json", base_path), &metadata)?;
        Ok(())
    }

    pub(crate) fn add_node(
        &mut self,
        engine: &DatabaseEngine,
        node: &GraphNode,
    ) -> Result<(), PackageWriterError> {
        match node {
            GraphNode::Change(change) => self.add_change(engine, change),
            GraphNode::Plan(plan) => self.add_plan_to_engine(engine, plan),
            GraphNode::Root => Ok(()),
        }
    }

    pub(crate) fn finalize(self) -> Result<(), PackageWriterError> {
        self.backend.finish()
    }
}
