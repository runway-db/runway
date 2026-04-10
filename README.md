# Runway
> Let your database take flight.

Runway is a modern, decentralized database migration tool designed for flexibility, safety, and multi-engine support. Unlike traditional migration tools that rely on a single, fragile sequence of files, Runway uses a Directed Acyclic Graph (DAG) to determine the correct execution order of your changes.

## Features

- **Decentralized Management**: No more merge conflicts on a central `migrations.sql` or numbering files `001`, `002`, etc.
- **DAG-based Discovery**: Changes explicitly state their requirements, and Runway calculates the correct order.
- **Multi-Engine Support**: Write engine-specific SQL (e.g., `deploy.postgres.sql` and `deploy.sqlite.sql`) within the same change.
- **Plans & Locking**: Group changes into named "Plans" (like releases) and lock them to ensure historical integrity via SHA-256 hashing.
- **ZIP Packaging**: Compile migrations into a portable ZIP archive for deployment.
- **Polyglot Ecosystem**: While the CLI is built in Rust, Runway is designed to be cross-platform. Packaged migrations can be consumed by libraries in Rust, .NET, JVM languages, Python, and more.
- **Sync & Async Runtime**: Use Runway as a standalone CLI or embed it into your application.

## Core Concepts

### Changes
A **Change** is a single unit of migration. It lives in its own directory under `changes/` and contains:
- `change.toml`: Defines the change's metadata and dependencies.
- `deploy.sql`: The SQL to apply the change.
- `revert.sql`: (Optional) The SQL to undo the change.
- `verify.sql`: (Optional) SQL to verify the change was applied correctly.

### Plans
A **Plan** is a collection of target changes and/or other plans. It represents a milestone in your database schema (e.g., `@v1.0`). Plans can be "locked," creating a `plan.lock` file that stores the hashes of all dependencies to prevent accidental tampering with applied history.

### The Graph
Runway scans your `changes/` and `plans/` directories, analyzes the `requires` fields, and builds a dependency graph. When you run `up`, Runway traverses this graph to ensure everything is applied in the correct order.

## Installation

```bash
cargo install --path .
```

## CLI Usage

### 1. Initialize a Project
Scaffold a new Runway project in the current directory:
```bash
runway init
```

### 2. Add a Change
Create a new migration named `create_users`:
```bash
runway add change create_users
```

### 3. Apply Migrations
Apply all pending migrations to a SQLite database:
```bash
runway up --engine sqlite --url "sqlite://dev.db"
```

### 4. Create a Plan
Group recent changes into a versioned plan:
```bash
runway add plan v1.0
```
Edit `plans/v1.0/plan.toml` to include your target changes.

### 5. Lock a Plan
Freeze the state of a plan for production:
```bash
runway lock v1.0
```

### 6. Package for Deployment
Bundle your migrations into a ZIP file for your production environment:
```bash
runway package --output migrations.zip
```

### 7. Revert Changes
Roll back the database to a specific state:
```bash
runway down --engine sqlite --url "sqlite://dev.db" --to create_users
```

## Integration

Runway can be used as a standalone CLI for any project, or embedded directly into your application.

### Rust Example (Library)

If you are using Rust, you can use the `runway` crate to run migrations on startup:

```rust
use runway::{Migrator, Package, adapters::Rusqlite};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = rusqlite::Connection::open("prod.db")?;
    let mut package = Package::load("migrations.zip")?;
    
    let adapter = Rusqlite::new(&conn);
    let mut migrator = Migrator::new(adapter, &mut package);
    
    migrator.apply()?;
    Ok(())
}
```

## Project Structure

- `runway.toml`: Project configuration (e.g., enabled engines).
- `changes/`: Directory containing individual change folders.
- `plans/`: Directory containing plan folders and lockfiles.
- `_runway_history`: Internal table created in your database to track applied migrations.

- [Package Format](docs/package-format.md): Technical details of the portable ZIP migration package.
- [Client Library Guide](docs/client-library-guide.md): Instructions for implementing Runway SDKs in other languages.

## License

MIT OR Apache-2.0
