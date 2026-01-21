pub mod adapters;
#[cfg(feature = "generation")]
pub mod build;
mod db;
pub mod errors;
pub mod migrator;
pub mod package;

#[doc(inline)]
pub use db::DatabaseEngine;

#[cfg(feature = "generation")]
pub use build::{load_project, package_migrations};

#[cfg(feature = "build")]
pub use build::{package_project, package_named_project};

pub use migrator::{MigrationSource, Migrator};

#[cfg(any(feature = "build", feature = "generation", feature = "cli"))]
pub use package::Package;

#[cfg(feature = "build")]
#[macro_export]
macro_rules! package {
    () => {
        $crate::package_project(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations"))
            .expect("Failed to package migrations")
    };
    ($name:literal) => {
        $crate::package_named_project(concat!(env!("CARGO_MANIFEST_DIR"), "/", $name), $name)
            .expect(concat!("Failed to package ", $name, " migrations"))
    };
    ($path:literal, $name:literal) => {
        $crate::package_named_project(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path), $name)
            .expect(concat!("Failed to package ", $name, " migrations"))
    };
}

#[cfg(feature = "build")]
#[macro_export]
macro_rules! embed_migrations {
    () => {
        $crate::Package::from_bytes(include_bytes!(concat!(env!("OUT_DIR"), "/migrations.runway")))
            .expect("Failed to load embedded migrations")
    };
    ($name:literal) => {
        $crate::Package::from_bytes(include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".runway")))
            .expect("Failed to load embedded migrations")
    };
}
