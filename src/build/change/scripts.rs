use crate::DatabaseEngine;
use crate::build::ScriptError;
use std::fs;
use std::path::{Path, PathBuf};

fn read_script(change_path: impl AsRef<Path>, script_name: &str) -> Result<Script, ScriptError> {
    let script_path = change_path.as_ref().join(script_name);
    log::debug!("Reading script from {:?}", script_path);

    let contents = fs::read_to_string(&script_path).map_err(|e| ScriptError::ReadFailed {
        path: script_path.clone(),
        source: e,
    })?;

    Ok(Script::new(script_path, contents))
}

fn script_name(script_name: &str, script_extension: &String) -> String {
    format!("{}.{}", script_name, script_extension)
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) struct Script {
    pub(super) path: PathBuf,
    pub(super) contents: String,
}

impl Script {
    fn new(path: PathBuf, contents: String) -> Self {
        Self { path, contents }
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct ScriptCollection {
    pub(super) deploy: Script,
    pub(super) revert: Option<Script>,
    pub(super) verify: Option<Script>,
}

impl ScriptCollection {
    pub(super) fn load_for_engine(
        change_path: impl AsRef<Path>,
        engine: &DatabaseEngine,
    ) -> Result<ScriptCollection, ScriptError> {
        let change_path = change_path.as_ref().to_path_buf();

        let extensions = {
            let engine_extension = match engine {
                DatabaseEngine::Postgres => "postgres.sql",
                DatabaseEngine::Sqlite => "sqlite.sql",
                DatabaseEngine::MSSQL => "mssql.sql",
                DatabaseEngine::MySQL => "mysql.sql",
            };
            vec![engine_extension.to_string(), "sql".to_string()]
        };

        let read_engine_specific = |script: &str| -> Result<Option<Script>, ScriptError> {
            for extension in &extensions {
                match read_script(&change_path, script_name(script, extension).as_str()) {
                    Ok(script) => return Ok(Some(script)),
                    Err(ScriptError::ReadFailed { source, .. })
                        if source.kind() == std::io::ErrorKind::NotFound =>
                    {
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }

            Ok(None)
        };

        let deploy = match read_engine_specific("deploy")? {
            Some(script) => script,
            None => return Err(ScriptError::MissingDeployScript { path: change_path }),
        };

        let revert = read_engine_specific("revert")?;
        let verify = read_engine_specific("verify")?;

        Ok(Self {
            deploy,
            revert,
            verify,
        })
    }

    pub fn deploy(&self) -> &String {
        &self.deploy.contents
    }

    pub fn revert(&self) -> Option<&String> {
        self.revert.as_ref().map(|s| &s.contents)
    }

    pub fn verify(&self) -> Option<&String> {
        self.verify.as_ref().map(|s| &s.contents)
    }
}
