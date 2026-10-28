---
name: db-migration
description: Adds or changes Tadidy's SQLite schema (SQLCipher) — an append-only rusqlite_migration step, the FTS5 triggers kept in sync, indexes for the queries, a test that migrates an empty database and a database at the previous version with data, and repository tests. Use for any ticket with a Database section, or whenever a table, column, index, trigger or virtual table changes.
---

# db-migration — one schema change, one tested migration

Model: [docs/spec/04-architecture.md#data-model](../../../docs/spec/04-architecture.md#data-model). Encryption: [#encryption](../../../docs/spec/04-architecture.md#encryption).

## Rules

- Migrations live in `src-tauri/src/storage/migrations.rs` as an ordered list of `rusqlite_migration::M::up(...)`; the schema version is SQLite's `user_version`.
- **Append-only**: never edit or reorder a migration that reached `main`. A mistake is fixed by a new migration.
- Every migration runs inside the encrypted connection (`PRAGMA key` first), in a transaction.
- SQLite cannot drop/alter many things in place: to change a column, create the new table, copy, drop, rename — in one migration, with foreign keys checked (`PRAGMA foreign_key_check`).
- `clips_fts` (FTS5, external content `clips`) is kept in sync by `AFTER INSERT/UPDATE/DELETE` triggers; a migration that touches `clips` columns indexed by FTS updates the triggers and rebuilds the index (`INSERT INTO clips_fts(clips_fts) VALUES('rebuild')`).
- Add the index each new query needs (`EXPLAIN QUERY PLAN` in a test or in the pull request).
- Data that must never be lost (pinned clips, snippets, collections) survives every migration: test it.

## Steps

1. **Red tests** in `src-tauri/tests/migrations.rs`:
   - `migrations_are_valid`: `MIGRATIONS.validate()` passes.
   - From empty: open a temporary encrypted database, migrate to latest, assert the new table / column / index exists (`sqlite_master`, `pragma_table_info`).
   - From the previous version **with data**: migrate to N-1, insert representative rows (a pinned clip, a snippet), migrate to N, assert the data is intact and transformed as expected.
   - Repository test of the query that needed the change.
   Commit `test(TAD-xxx): …`.
2. **Write the migration** (SQL in a `const` or `include_str!("sql/000N_<name>.sql")`), then the repository code. Commit `feat(TAD-xxx): …`.
3. **Update the spec** if the model changed (`docs/spec/04-architecture.md#data-model`), keeping its anchors, and rebuild `spec.html`.
4. `scripts/check.sh --cov`.

## Checklist for the pull request

- [ ] New migration appended, previous ones untouched (`git diff main -- src-tauri/src/storage/migrations.rs` only adds).
- [ ] Tested from empty and from N-1 with data; pinned clips and snippets survive.
- [ ] FTS triggers and index consistent; `EXPLAIN QUERY PLAN` uses the intended index.
- [ ] No clip content in migration logs.
