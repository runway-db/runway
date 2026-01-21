use crate::build::project::graph::ChangeGraphError;
use std::path::PathBuf;
use wherror::Error;

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("IO error at {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Configuration error in {path:?}: {source}")]
    Config {
        path: PathBuf,
        #[source]
        source: DescriptorError,
    },

    #[error("Error in change '{name}': {source}")]
    Change {
        name: String,
        #[source]
        source: ChangeError,
    },

    #[error("Error in plan '{name}': {source}")]
    Plan {
        name: String,
        #[source]
        source: PlanError,
    },

    #[error("Dependency graph error: {0}")]
    Graph(#[from] ChangeGraphError),

    #[error("Script error: {0}")]
    Script(#[from] ScriptError),

    #[error("Packaging error: {0}")]
    Package(String),

    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Error)]
pub enum ChangeError {
    #[error("Failed to load descriptor: {source}")]
    Descriptor {
        #[source]
        source: DescriptorError,
    },

    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

#[derive(Debug, Error)]
pub enum PlanError {
    #[error("Failed to load descriptor: {source}")]
    Descriptor {
        #[source]
        source: DescriptorError,
    },

    #[error("Failed to load lock: {source}")]
    Lock {
        #[source]
        source: DescriptorError,
    },

    #[error("Lock mismatch for {target} in engine {engine}: expected {expected}, found {found}")]
    LockMismatch {
        target: String,
        engine: crate::DatabaseEngine,
        expected: String,
        found: String,
    },

    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("Missing deploy script at {path:?}")]
    MissingDeployScript { path: PathBuf },

    #[error("Failed to read script at {path:?}: {source}")]
    ReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
pub enum DescriptorError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

impl From<std::io::Error> for BuildError {
    fn from(source: std::io::Error) -> Self {
        Self::Io {
            path: PathBuf::new(), // We don't have the path here, but this is a fallback
            source,
        }
    }
}
