use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, Transaction};

use super::{MigrationError, PersistenceError};
use crate::domain::plant_bundle::validate_bundle;

pub const CURRENT_SCHEMA_VERSION: i64 = 5;

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
    Migration {
        version: 3,
        name: "003_session_plant_preview",
        apply: apply_session_plant_preview,
    },
    Migration {
        version: 4,
        name: "004_transactional_plants",
        apply: apply_transactional_plants,
    },
    Migration {
        version: 5,
        name: "005_app_settings",
        apply: apply_app_settings,
    },
];

#[cfg(test)]
pub(crate) fn registered_migrations_for_tests() -> &'static [Migration] {
    MIGRATIONS
}

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
            // This is deliberately inside the loop.  A v1/v2 database gets
            // its v3 columns first, then is checked *before* 004 can write
            // anything, exactly like a database that started at v3.
            if migration.version == 4 {
                preflight_v3_generation_data(connection)?;
            }
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

/// Read-only v4 reconciliation.  It intentionally never creates a missing
/// plant: a completed session without its transactional receipt is unsafe.
pub fn audit_transactional_data(connection: &Connection) -> Result<(), PersistenceError> {
    let version = migration_history_versions(connection)?
        .last()
        .copied()
        .unwrap_or_default();
    if version < 4 {
        return Ok(());
    }
    let foreign_keys: i64 = connection
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .map_err(PersistenceError::DatabaseOperation)?;
    if foreign_keys != 0 {
        return Err(PersistenceError::DataSafetyBlocker);
    }
    let mut statement = connection.prepare(
        "SELECT s.id, s.status, s.completed_at, s.plant_seed, s.generator_version, s.plant_genome_json,
                p.id, p.planted_at, p.plant_seed, p.generator_version, p.plant_genome_json
         FROM sessions s LEFT JOIN plants p ON p.source_session_id = s.id"
    ).map_err(PersistenceError::DatabaseOperation)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, Option<String>>(10)?,
            ))
        })
        .map_err(PersistenceError::DatabaseOperation)?;
    for row in rows {
        let (
            _id,
            status,
            completed_at,
            seed,
            gen,
            json,
            plant_id,
            planted_at,
            plant_seed,
            plant_gen,
            plant_json,
        ) = row.map_err(PersistenceError::DatabaseOperation)?;
        match (seed.as_deref(), gen, json.as_deref()) {
            (None, None, None) if status != "COMPLETED" => {}
            (Some(seed), Some(version), Some(json))
                if validate_bundle(seed, version, json).is_ok() => {}
            _ => return Err(PersistenceError::DataSafetyBlocker),
        }
        if status == "COMPLETED" {
            if plant_id.is_none()
                || completed_at != planted_at
                || seed != plant_seed
                || gen != plant_gen
                || json != plant_json
            {
                return Err(PersistenceError::DataSafetyBlocker);
            }
        } else if plant_id.is_some() {
            return Err(PersistenceError::DataSafetyBlocker);
        }
    }
    Ok(())
}

fn preflight_v3_generation_data(connection: &Connection) -> Result<(), PersistenceError> {
    let mut statement = connection
        .prepare("SELECT status, plant_seed, generator_version, plant_genome_json FROM sessions")
        .map_err(PersistenceError::DatabaseOperation)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(PersistenceError::DatabaseOperation)?;
    for row in rows {
        let (status, seed, version, genome) = row.map_err(PersistenceError::DatabaseOperation)?;
        let complete_bundle = matches!((&seed, version, &genome), (Some(_), Some(_), Some(_)));
        match (seed.as_deref(), version, genome.as_deref()) {
            (None, None, None) if status != "COMPLETED" => {}
            (Some(seed), Some(version), Some(genome)) => {
                if validate_bundle(seed, version, genome).is_err()
                    || status == "COMPLETED" && seed.is_empty()
                {
                    return Err(PersistenceError::DataSafetyBlocker);
                }
            }
            _ => return Err(PersistenceError::DataSafetyBlocker),
        }
        if status == "COMPLETED" && !complete_bundle {
            return Err(PersistenceError::DataSafetyBlocker);
        }
    }
    Ok(())
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

fn apply_session_plant_preview(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    // SQLite cannot add a table-level CHECK with ALTER TABLE. Keep the existing
    // table and data intact, then enforce the nullable bundle through triggers.
    transaction.execute_batch(
        "ALTER TABLE sessions ADD COLUMN plant_seed TEXT;
         ALTER TABLE sessions ADD COLUMN generator_version INTEGER;
         ALTER TABLE sessions ADD COLUMN plant_genome_json TEXT;
         CREATE TRIGGER sessions_generation_bundle_insert
         BEFORE INSERT ON sessions
         WHEN (NEW.plant_seed IS NULL OR NEW.generator_version IS NULL OR NEW.plant_genome_json IS NULL)
          AND NOT (NEW.plant_seed IS NULL AND NEW.generator_version IS NULL AND NEW.plant_genome_json IS NULL)
         BEGIN
           SELECT RAISE(ABORT, 'session generation bundle must be all-null or all-present');
         END;
         CREATE TRIGGER sessions_generation_bundle_update
         BEFORE UPDATE OF plant_seed, generator_version, plant_genome_json ON sessions
         WHEN (NEW.plant_seed IS NULL OR NEW.generator_version IS NULL OR NEW.plant_genome_json IS NULL)
          AND NOT (NEW.plant_seed IS NULL AND NEW.generator_version IS NULL AND NEW.plant_genome_json IS NULL)
         BEGIN
           SELECT RAISE(ABORT, 'session generation bundle must be all-null or all-present');
         END;",
    )
}

fn apply_transactional_plants(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute_batch(
        "CREATE TABLE plants (
            id TEXT PRIMARY KEY NOT NULL,
            source_session_id TEXT NOT NULL UNIQUE REFERENCES sessions(id) ON DELETE RESTRICT ON UPDATE RESTRICT,
            plant_seed TEXT NOT NULL CHECK (length(plant_seed) = 16 AND plant_seed NOT GLOB '*[^0-9a-f]*'),
            generator_version INTEGER NOT NULL CHECK (generator_version > 0),
            plant_genome_json TEXT NOT NULL CHECK (json_valid(plant_genome_json)),
            planted_at INTEGER NOT NULL CHECK (planted_at >= 0)
        );
        CREATE INDEX plants_planted_at_id ON plants (planted_at DESC, id DESC);
        INSERT INTO plants (id, source_session_id, plant_seed, generator_version, plant_genome_json, planted_at)
          SELECT id, id, plant_seed, generator_version, plant_genome_json, completed_at
          FROM sessions WHERE status = 'COMPLETED';
        CREATE TRIGGER plants_must_exactly_match_completed_session
        BEFORE INSERT ON plants
        WHEN NOT EXISTS (SELECT 1 FROM sessions s WHERE s.id = NEW.source_session_id
          AND s.status = 'COMPLETED' AND s.completed_at = NEW.planted_at
          AND s.plant_seed = NEW.plant_seed AND s.generator_version = NEW.generator_version
          AND s.plant_genome_json = NEW.plant_genome_json)
        BEGIN SELECT RAISE(ABORT, 'plant must exactly match completed source session'); END;
        CREATE TRIGGER plants_are_immutable_update BEFORE UPDATE ON plants
        BEGIN SELECT RAISE(ABORT, 'plants are immutable'); END;
        CREATE TRIGGER plants_are_immutable_delete BEFORE DELETE ON plants
        BEGIN SELECT RAISE(ABORT, 'plants are immutable'); END;
        CREATE TRIGGER completed_plants_protect_session
        BEFORE UPDATE OF status, completed_at, plant_seed, generator_version, plant_genome_json ON sessions
        WHEN EXISTS (SELECT 1 FROM plants p WHERE p.source_session_id = OLD.id)
        BEGIN SELECT RAISE(ABORT, 'completed plant source session is immutable'); END;"
    )
}

fn apply_app_settings(transaction: &Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute_batch(
        "CREATE TABLE app_settings (
            singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
            default_focus_duration_ms INTEGER NOT NULL CHECK (default_focus_duration_ms BETWEEN 60000 AND 10800000)
        );
        INSERT INTO app_settings (singleton, default_focus_duration_ms) VALUES (1, 1500000);",
    )
}
