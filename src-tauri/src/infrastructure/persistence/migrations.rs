use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, Transaction};

use super::{MigrationError, PersistenceError};

pub const CURRENT_SCHEMA_VERSION: i64 = 2;

#[derive(Clone, Copy)]
pub(crate) struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub apply: for<'transaction> fn(&Transaction<'transaction>) -> rusqlite::Result<()>,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "001_persistence_foundation",
        apply: apply_persistence_foundation,
    },
    Migration {
        version: 2,
        name: "002_focus_sessions",
        apply: apply_focus_sessions,
    },
];

pub fn run_migrations(connection: &mut Connection) -> Result<i64, PersistenceError> {
    run_migrations_with(connection, MIGRATIONS, CURRENT_SCHEMA_VERSION)
}

pub(crate) fn run_migrations_with(
    connection: &mut Connection,
    migrations: &[Migration],
    current_schema_version: i64,
) -> Result<i64, PersistenceError> {
    bootstrap_migration_tracking(connection)?;
    let recorded_versions = migration_history_versions(connection)?;
    let database_version = recorded_versions.last().copied().unwrap_or_default();

    if database_version > current_schema_version {
        return Err(PersistenceError::UnsupportedFutureSchema {
            database_version,
            current_version: current_schema_version,
        });
    }

    validate_migration_history(&recorded_versions)?;

    for migration in migrations {
        if migration.version > database_version {
            apply_migration(connection, migration)?;
        }
    }

    let final_versions = migration_history_versions(connection)?;
    validate_migration_history(&final_versions)?;
    let final_version = final_versions.last().copied().unwrap_or_default();
    if final_version != current_schema_version {
        return Err(PersistenceError::Migration(MigrationError::Failed {
            version: current_schema_version,
            name: "migration_registry",
            source: rusqlite::Error::InvalidQuery,
        }));
    }
    Ok(final_version)
}

fn bootstrap_migration_tracking(connection: &Connection) -> Result<(), PersistenceError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at INTEGER NOT NULL
            );",
        )
        .map_err(MigrationError::Bootstrap)?;
    Ok(())
}

fn migration_history_versions(connection: &Connection) -> Result<Vec<i64>, PersistenceError> {
    let mut statement = connection
        .prepare("SELECT version FROM schema_migrations ORDER BY version ASC")
        .map_err(PersistenceError::DatabaseOperation)?;
    let versions = statement
        .query_map([], |row| row.get(0))
        .map_err(PersistenceError::DatabaseOperation)?
        .collect::<Result<Vec<i64>, _>>()
        .map_err(PersistenceError::DatabaseOperation)?;
    Ok(versions)
}

fn validate_migration_history(versions: &[i64]) -> Result<(), PersistenceError> {
    let is_continuous_prefix = versions
        .iter()
        .enumerate()
        .all(|(index, version)| *version == index as i64 + 1);

    if is_continuous_prefix {
        Ok(())
    } else {
        Err(PersistenceError::MigrationHistoryInvalid)
    }
}

fn apply_migration(
    connection: &mut Connection,
    migration: &Migration,
) -> Result<(), PersistenceError> {
    let transaction = connection
        .transaction()
        .map_err(|source| MigrationError::Failed {
            version: migration.version,
            name: migration.name,
            source,
        })?;
    (migration.apply)(&transaction).map_err(|source| MigrationError::Failed {
        version: migration.version,
        name: migration.name,
        source,
    })?;
    transaction
        .execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![migration.version, migration.name, utc_epoch_millis()?],
        )
        .map_err(|source| MigrationError::Failed {
            version: migration.version,
            name: migration.name,
            source,
        })?;
    transaction
        .commit()
        .map_err(|source| MigrationError::Failed {
            version: migration.version,
            name: migration.name,
            source,
        })?;
    Ok(())
}

fn utc_epoch_millis() -> Result<i64, PersistenceError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(PersistenceError::Clock)?
        .as_millis() as i64)
}

fn apply_persistence_foundation(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute_batch(
        "CREATE TABLE app_metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )
}

fn apply_focus_sessions(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute_batch(
        "CREATE TABLE sessions (
            id TEXT PRIMARY KEY NOT NULL,
            planned_duration_ms INTEGER NOT NULL CHECK (planned_duration_ms > 0),
            started_at INTEGER NOT NULL,
            target_end_at INTEGER NOT NULL,
            paused_at INTEGER,
            total_paused_ms INTEGER NOT NULL DEFAULT 0 CHECK (total_paused_ms >= 0),
            completed_at INTEGER,
            status TEXT NOT NULL CHECK (status IN ('ACTIVE', 'PAUSED', 'COMPLETED', 'ABANDONED', 'RECOVERY_REQUIRED')),
            pause_count INTEGER NOT NULL DEFAULT 0 CHECK (pause_count >= 0),
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            CHECK (target_end_at >= started_at),
            CHECK ((status = 'PAUSED' AND paused_at IS NOT NULL) OR (status != 'PAUSED' AND paused_at IS NULL)),
            CHECK ((status = 'COMPLETED' AND completed_at IS NOT NULL) OR (status != 'COMPLETED' AND completed_at IS NULL))
        );
        CREATE UNIQUE INDEX sessions_single_open_session
            ON sessions ((1))
            WHERE status IN ('ACTIVE', 'PAUSED', 'RECOVERY_REQUIRED');
        CREATE INDEX sessions_status_created_at ON sessions (status, created_at DESC);",
    )
}
