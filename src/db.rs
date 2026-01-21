use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, Hash, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[non_exhaustive]
pub enum DatabaseEngine {
    #[serde(rename = "postgres")]
    #[serde(alias = "postgresql")]
    Postgres,
    #[serde(rename = "sqlite")]
    Sqlite,
    #[serde(rename = "mssql")]
    #[serde(alias = "sqlserver")]
    MSSQL,
    #[serde(rename = "mysql")]
    #[serde(alias = "mariadb")]
    MySQL,
}

impl DatabaseEngine {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Postgres => "postgres",
            Self::Sqlite => "sqlite",
            Self::MySQL => "mysql",
            Self::MSSQL => "mssql",
        }
    }

    pub fn identifier(&self) -> &str {
        match self {
            Self::Postgres => "pg",
            Self::Sqlite => "sqlite",
            Self::MySQL => "mysql",
            Self::MSSQL => "mssql",
        }
    }
}

impl Display for DatabaseEngine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DatabaseEngine {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "postgres" | "postgresql" | "pg" => Ok(Self::Postgres),
            "sqlite" => Ok(Self::Sqlite),
            "mysql" | "mariadb" => Ok(Self::MySQL),
            "mssql" | "sqlserver" => Ok(Self::MSSQL),
            _ => Err(format!("Unknown database engine: {}", s)),
        }
    }
}
