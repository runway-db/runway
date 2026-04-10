# Client Library Implementation Guide

This guide is intended for developers who want to implement a Runway client library (SDK) in a new language (e.g., .NET, Java, Python). 

Runway's core philosophy is that the complex dependency resolution (DAG) and packaging logic should be handled by the Rust CLI, while client libraries remain simple, reliable consumers of the packaged migrations.

## High-Level Responsibilities

A Runway client library has four primary responsibilities:
1. **Load the Package**: Read and parse the ZIP migration package.
2. **Connect to the Database**: Use the language's native database drivers.
3. **Manage the History Table**: Initialize and query the `_runway_history` table.
4. **Apply/Revert Migrations**: Execute the SQL scripts in the order defined by the package.

---

## 1. Loading the Package

The client library should provide a way to load a Runway `.zip` package.

- Parse `package.json` to identify supported engines.
- Provide access to engine-specific `metadata.json` (the execution sequence).
- Provide access to change-specific `metadata.json` (name and hash).
- Load SQL scripts (`deploy.sql`, `revert.sql`, `verify.sql`) for a given change.

Refer to the [Package Format Specification](package-format.md) for details on the ZIP structure.

## 2. The History Table (`_runway_history`)

Every database managed by Runway must have a history table. The client library is responsible for ensuring this table exists.

### Schema Requirements:
The schema should be consistent across all implementations.

| Column | Type | Description |
| :--- | :--- | :--- |
| `id` | Serial/Auto-inc | Primary key. |
| `name` | Text | The name of the change or plan marker (e.g., `create_users` or `@v1.0`). |
| `type` | Text | Either `'change'` or `'plan'`. |
| `hash` | Text | The SHA-256 hash of the change at the time it was applied. |
| `applied_at` | Timestamp | When the migration was applied. Should have a `DEFAULT CURRENT_TIMESTAMP`. |
| `success` | Boolean | Whether the migration succeeded. |
| `execution_time` | Integer | (Nullable) Time in milliseconds. |
| `reverted_at` | Timestamp | (Nullable) When the migration was reverted. |
| `runway_version` | Text | The version of Runway (or client library) that applied it. |

*Note: `TIMESTAMPTZ` is preferred for Postgres and `DATETIME` for SQLite to maintain consistency.*

## 3. The Migration Logic (Applying)

When `Apply(target)` is called, the library should follow these steps:

1. **Initialize**: Ensure the `_runway_history` table exists.
2. **Fetch Sequence**: Load the `sequence` from `engines/{engine}/metadata.json` in the package.
3. **Handle Target**: If a `target` is provided, truncate the sequence so it ends at the specified migration. If the target is not found, throw an error.
4. **Loop through Sequence**: For each `item` in the sequence:
   - **Consolidated History Check**: Query the history table for the latest non-reverted record for `item` (including its name, type, hash, and success status).
   - **Check if Applied**:
     - If a record exists and `success` is true:
       - **Verify Hash**: If it's a `change`, compare the package's hash with the recorded hash. Throw an error on mismatch.
       - Skip to the next item.
     - If a record exists and `success` is false:
       - Log a warning that the migration is being retried.
   - **Execute Scripts**:
     - If it's a `plan` marker (prefixed with `@`):
       - Plans have no scripts. Mark as successful immediately.
     - If it's a `change`:
       - Load `deploy.sql`. Execute it.
       - Load `verify.sql` (if it exists). Execute it.
   - **Record Result**: Insert a new row into `_runway_history`. If any script fails, `success` should be false, and execution must stop.

## 4. The Migration Logic (Reverting)

When `Revert(target)` is called:

1. **Identify Targets**: Reverse the execution sequence.
2. **Filter**: If `target` is null, revert all successful migrations. If `target` is provided, identify all successful migrations that appear *after* the `target` in the sequence.
3. **Loop through Reversed Sequence**:
   - **Check Applied**: Skip items that were never successfully applied or have already been reverted.
   - **Plan Marker Handling**: Plan markers (prefixed with `@`) are skipped during reversion. They have no scripts and should **not** be marked as reverted.
   - **Load Scripts**: Load `revert.sql` for the change.
   - **Execute SQL**: If `revert.sql` exists, execute it.
   - **Mark Reverted**: If `revert.sql` was executed, update the history table, setting `reverted_at` to the current timestamp for the most recent successful record of that change.
   - **Handle Missing Scripts**: If a change has no `revert.sql`, log a warning and do **not** mark it as reverted (ensuring it remains "applied" logically).

## 5. Implementation Patterns

We recommend following the "Adapter" or "Provider" pattern.

- **`MigrationSource`**: An interface/trait for accessing package contents (ZIP vs. Directory).
  - Should distinguish between required scripts (`deploy.sql`) and optional scripts (`verify.sql`, `revert.sql`).
- **`MigrationAdapter`**: An interface/trait for database-specific operations (executing SQL, managing history table).
- **`Migrator`**: The high-level orchestrator that uses the Source and Adapter.

---

By following these guidelines, you ensure that migrations applied by your library are fully compatible with those applied by the Runway CLI and other language SDKs.
