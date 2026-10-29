//! The schema, as an ordered list of migrations. Append-only: a migration that reached `main` is
//! never edited, a mistake is fixed by a new one (skill `db-migration`). The schema version is
//! SQLite's `user_version`.

use rusqlite_migration::{M, Migrations};

const STEPS: &[M<'static>] = &[
    M::up(include_str!("migrations/0001_settings.sql")),
    M::up(include_str!("migrations/0002_clips.sql")),
];

pub static MIGRATIONS: Migrations<'static> = Migrations::from_slice(STEPS);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        assert_eq!(MIGRATIONS.validate(), Ok(()));
    }
}
