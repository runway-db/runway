use crate::DatabaseEngine;
use crate::errors::PackageError;
use crate::package::metadata::{ChangeMetadata, EngineMetadata, PackageMetadata};
use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

pub mod metadata;

#[cfg(feature = "generation")]
pub(crate) mod writer;

#[cfg(feature = "generation")]
pub(crate) use writer::{DirectoryWriterBackend, PackageWriter, PackageWriterBackend, ZipWriterBackend};

pub(crate) trait PackageBackend {
    fn read_file(&mut self, path: &str) -> Result<String, PackageError>;
    fn read_file_optional(&mut self, path: &str) -> Result<Option<String>, PackageError>;
}

struct ZipBackend<R: Read + Seek> {
    archive: ZipArchive<R>,
}

impl<R: Read + Seek> PackageBackend for ZipBackend<R> {
    fn read_file(&mut self, path: &str) -> Result<String, PackageError> {
        let mut file = self.archive.by_name(path).map_err(|e| match e {
            zip::result::ZipError::FileNotFound => PackageError::MissingFile(path.to_string()),
            _ => PackageError::Zip(e),
        })?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| PackageError::Io {
            path: PathBuf::from(path),
            source: e,
        })?;
        Ok(contents)
    }

    fn read_file_optional(&mut self, path: &str) -> Result<Option<String>, PackageError> {
        match self.archive.by_name(path) {
            Ok(mut file) => {
                let mut contents = String::new();
                file.read_to_string(&mut contents).map_err(|e| PackageError::Io {
                    path: PathBuf::from(path),
                    source: e,
                })?;
                Ok(Some(contents))
            }
            Err(zip::result::ZipError::FileNotFound) => Ok(None),
            Err(e) => Err(PackageError::Zip(e)),
        }
    }
}

struct DirectoryBackend {
    root: PathBuf,
}

impl PackageBackend for DirectoryBackend {
    fn read_file(&mut self, path: &str) -> Result<String, PackageError> {
        let full_path = self.root.join(path);
        std::fs::read_to_string(&full_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                PackageError::MissingFile(path.to_string())
            } else {
                PackageError::Io {
                    path: full_path,
                    source: e,
                }
            }
        })
    }

    fn read_file_optional(&mut self, path: &str) -> Result<Option<String>, PackageError> {
        let full_path = self.root.join(path);
        match std::fs::read_to_string(&full_path) {
            Ok(contents) => Ok(Some(contents)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(PackageError::Io {
                path: full_path,
                source: e,
            }),
        }
    }
}

pub struct Package {
    metadata: PackageMetadata,
    backend: Box<dyn PackageBackend>,
}

impl Package {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PackageError> {
        let path = path.as_ref();
        let backend: Box<dyn PackageBackend> = if path.is_dir() {
            Box::new(DirectoryBackend {
                root: path.to_path_buf(),
            })
        } else {
            let file = File::open(path).map_err(|e| PackageError::Io {
                path: path.to_path_buf(),
                source: e,
            })?;

            let archive = ZipArchive::new(file)?;
            Box::new(ZipBackend { archive })
        };

        Self::load_from_backend(backend)
    }

    pub fn from_bytes(bytes: &'static [u8]) -> Result<Self, PackageError> {
        let cursor = std::io::Cursor::new(bytes);
        let archive = ZipArchive::new(cursor)?;
        let backend = Box::new(ZipBackend { archive });

        Self::load_from_backend(backend)
    }

    fn load_from_backend(mut backend: Box<dyn PackageBackend>) -> Result<Self, PackageError> {
        let metadata_contents = backend.read_file("package.json")?;
        let metadata = serde_json::from_str(&metadata_contents)?;

        Ok(Self { metadata, backend })
    }

    pub fn engines(&self) -> &[DatabaseEngine] {
        self.metadata.engines()
    }

    pub fn engine_metadata(
        &mut self,
        engine: &DatabaseEngine,
    ) -> Result<EngineMetadata, PackageError> {
        let path = format!("engines/{}/metadata.json", engine.identifier());
        let contents = self.backend.read_file(&path)?;
        Ok(serde_json::from_str(&contents)?)
    }

    pub fn change_metadata(
        &mut self,
        engine: &DatabaseEngine,
        change_name: &str,
    ) -> Result<ChangeMetadata, PackageError> {
        let path = format!(
            "engines/{}/{}/metadata.json",
            engine.identifier(),
            change_name
        );
        let contents = self.backend.read_file(&path)?;
        Ok(serde_json::from_str(&contents)?)
    }

    pub fn deploy_script(
        &mut self,
        engine: &DatabaseEngine,
        change_name: &str,
    ) -> Result<String, PackageError> {
        let path = format!(
            "engines/{}/{}/deploy.sql",
            engine.identifier(),
            change_name
        );
        self.backend.read_file(&path)
    }

    pub fn revert_script(
        &mut self,
        engine: &DatabaseEngine,
        change_name: &str,
    ) -> Result<Option<String>, PackageError> {
        let path = format!(
            "engines/{}/{}/revert.sql",
            engine.identifier(),
            change_name
        );
        self.backend.read_file_optional(&path)
    }

    pub fn verify_script(
        &mut self,
        engine: &DatabaseEngine,
        change_name: &str,
    ) -> Result<Option<String>, PackageError> {
        let path = format!(
            "engines/{}/{}/verify.sql",
            engine.identifier(),
            change_name
        );
        self.backend.read_file_optional(&path)
    }
}


impl crate::migrator::MigrationSource for Package {
    fn engine_metadata(&mut self, engine: &DatabaseEngine) -> Result<EngineMetadata, Box<dyn std::error::Error>> {
        Ok(self.engine_metadata(engine)?)
    }

    fn change_metadata(&mut self, engine: &DatabaseEngine, change_name: &str) -> Result<ChangeMetadata, Box<dyn std::error::Error>> {
        Ok(self.change_metadata(engine, change_name)?)
    }

    fn deploy_script(&mut self, engine: &DatabaseEngine, change_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.deploy_script(engine, change_name)?)
    }

    fn verify_script(&mut self, engine: &DatabaseEngine, change_name: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        Ok(self.verify_script(engine, change_name)?)
    }

    fn revert_script(&mut self, engine: &DatabaseEngine, change_name: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        Ok(self.revert_script(engine, change_name)?)
    }
}

