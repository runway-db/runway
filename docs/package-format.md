# Runway Package Format

Runway migrations are compiled into a portable ZIP archive. This format is designed to be language-agnostic, allowing Runway's execution logic to be implemented in various languages while consuming the same migration packages.

## Structure

The internal structure of a `.zip` package looks like this:

```text
package.json
engines/
  sqlite/
    metadata.json
    create_users/
      metadata.json
      deploy.sql
      revert.sql
      verify.sql
    @v1.0/
      metadata.json
  postgres/
    metadata.json
    ...
```

## Root Metadata (`package.json`)

This file contains global metadata for the package.

```json
{
  "engines": ["sqlite", "postgres"]
}
```

- `engines`: A list of database engine identifiers supported by this package.

## Engine Metadata (`engines/{engine}/metadata.json`)

Each engine directory contains a `metadata.json` which defines the exact sequence in which migrations must be applied for that specific database engine.

```json
{
  "sequence": ["create_users", "@v1.0", "add_email_to_users"]
}
```

- `sequence`: An ordered list of change names or plan markers (prefixed with `@`). Runway will apply these in the order provided.

## Change Metadata (`engines/{engine}/{change_name}/metadata.json`)

Every change within an engine has its own metadata file.

```json
{
  "name": "create_users",
  "hash": "f9f1e4e1eb52aadc205243aec74fe6c60239ccb6ee006a83f710e3d7d5c23ad5"
}
```

- `name`: The name of the change.
- `hash`: The SHA-256 hash of the change's contents (including scripts and requirements) at the time of packaging. This is used to verify integrity against the database's history table.

## Change Scripts

Within a change directory, the following SQL files may exist:

- `deploy.sql`: (Required) The SQL script to apply the change.
- `revert.sql`: (Optional) The SQL script to undo the change.
- `verify.sql`: (Optional) The SQL script to verify that the change was successfully applied.

## Plan Metadata (`engines/{engine}/@{plan_name}/metadata.json`)

Plans are represented as markers in the execution sequence.

```json
{
  "name": "v1.0",
  "hash": "..."
}
```

- `name`: The name of the plan (without the `@` prefix).
- `hash`: (Optional) The hash of the plan, often representing the state of its dependencies.

## Implementation Notes for Libraries

Libraries implementing a Runway-compatible migrator should:

1.  **Read `package.json`**: Verify the target engine is supported.
2.  **Read Engine `metadata.json`**: Load the execution sequence.
3.  **Check History**: Query the `_runway_history` table in the target database.
4.  **Iterate Sequence**:
    - If an item is already successfully applied (and hashes match), skip it.
    - If it's a new Change, execute `deploy.sql` and then `verify.sql`.
    - Record the result (success/failure, timing, hash) in `_runway_history`.
    - If it's a Plan marker, record its application in the history table.

## Empty vs. Missing Files

While the Runway CLI's packager ensures that necessary files exist, consumer libraries should handle files gracefully:
- **`deploy.sql`**: Generally expected to exist and be non-empty for `change` items.
- **`verify.sql` / `revert.sql`**: These are optional. If a script is missing from the package or is empty, it should be treated as a no-op (success) without throwing an error.
- **Plans**: Plan markers (prefixed with `@`) never have SQL scripts.
