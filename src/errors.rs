
use std::path::PathBuf;
use wherror::Error;

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("IO error at {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Missing file in package: {0}")]
    MissingFile(String),
}
