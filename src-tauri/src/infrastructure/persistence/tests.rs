use rusqlite::params;
use tempfile::tempdir;

use super::{
    migrations::{run_migrations_with, Migration},
    Database, PersistenceError, CURRENT_SCHEMA_VERSION,
};

fn test_database() -> (tempfile::TempDir, Database) {
    let directory = tempdir().expect("test temporary directory should be created");
    let database = Database::new(directory.path().join("persistence-test.sqlite3"));
    (directory, database)
}

#[test]
fn empty_database_applies_migration_chain() {
    let (_directory, database) = test_database();
    database.initialize().expect("migration should succeed");

    let connection = database.open_connection().expect("database should open");
    let version: i64 = connection
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("schema version should exist");
    let migration_name: String = connection
        .query_row(
            "SELECT name FROM schema_migrations WHERE version = 1",
            [],
            |row| row.get(0),
        )
        .expect("migration one should be recorded");

    assert_eq!(version, CURRENT_SCHEMA_VERSION);
    assert_eq!(migration_name, "001_persistence_foundation");
    let migration_two_name: String = connection
        .query_row(
            "SELECT name FROM schema_migrations WHERE version = 2",
            [],
            |row| row.get(0),
        )
        .expect("migration two should be recorded");
    assert_eq!(migration_two_name, "002_focus_sessions");
    let versions: Vec<i64> = connection
        .prepare("SELECT version FROM schema_migrations ORDER BY version")
        .expect("migration history query should prepare")
        .query_map([], |row| row.get(0))
        .expect("migration history query should run")
        .collect::<Result<Vec<i64>, _>>()
        .expect("migration history should decode");
    assert_eq!(versions, (1..=CURRENT_SCHEMA_VERSION).collect::<Vec<_>>());
}

#[test]
fn migrations_are_idempotent() {
    let (_directory, database) = test_database();
    database
        .initialize()
        .expect("initial migration should succeed");
    database
        .initialize()
        .expect("repeated migration should succeed");

    let connection = database.open_connection().expect("database should open");
    let migration_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("migration count should be readable");
    let table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'app_metadata'",
            [],
            |row| row.get(0),
        )
        .expect("table list should be readable");

    assert_eq!(migration_count, 2);
    assert_eq!(table_count, 1);
}

#[test]
fn future_schema_is_rejected_without_downgrade() {
    let (_directory, database) = test_database();
    database
        .initialize()
        .expect("initial migration should succeed");
    let connection = database.open_connection().expect("database should open");
    connection
        .execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![CURRENT_SCHEMA_VERSION + 1, "future", 0_i64],
        )
        .expect("future version fixture should be inserted");

    let result = database.initialize();
    assert!(matches!(
        result,
        Err(PersistenceError::UnsupportedFutureSchema { .. })
    ));
}

#[test]
fn gapped_history_is_rejected_before_pending_migrations_run() {
    let (_directory, database) = test_database();
    let mut connection = database.open_connection().expect("database should open");
    seed_migration_history(&connection, &[1, 3]);
    let fixture_registry = test_migration_registry();

    let result = run_migrations_with(&mut connection, &fixture_registry, 3);

    assert!(matches!(
        result,
        Err(PersistenceError::MigrationHistoryInvalid)
    ));
    assert!(!table_exists(&connection, "migration_two_applied"));
}

#[test]
fn history_without_migration_one_is_rejected() {
    let (_directory, database) = test_database();
    let mut connection = database.open_connection().expect("database should open");
    seed_migration_history(&connection, &[2]);
    let fixture_registry = test_migration_registry();

    let result = run_migrations_with(&mut connection, &fixture_registry, 3);

    assert!(matches!(
        result,
        Err(PersistenceError::MigrationHistoryInvalid)
    ));
}

#[test]
fn continuous_history_allows_the_next_test_migration() {
    let (_directory, database) = test_database();
    let mut connection = database.open_connection().expect("database should open");
    let fixture_registry = test_migration_registry();

    run_migrations_with(&mut connection, &fixture_registry[..2], 2)
        .expect("the first two fixture migrations should apply");
    run_migrations_with(&mut connection, &fixture_registry, 3)
        .expect("the continuous history should allow fixture migration three");

    let versions: Vec<i64> = connection
        .prepare("SELECT version FROM schema_migrations ORDER BY version ASC")
        .expect("migration history query should prepare")
        .query_map([], |row| row.get(0))
        .expect("migration history query should run")
        .collect::<Result<Vec<i64>, _>>()
        .expect("migration history rows should decode");

    assert_eq!(versions, vec![1, 2, 3]);
    assert!(table_exists(&connection, "migration_three_applied"));
}

#[test]
fn failing_migration_rolls_back_schema_and_tracking_record() {
    let (_directory, database) = test_database();
    let mut connection = database.open_connection().expect("database should open");
    let failing_registry = [Migration {
        version: 1,
        name: "001_failing_fixture",
        apply: failing_migration,
    }];

    let result = run_migrations_with(&mut connection, &failing_registry, 1);
    assert!(result.is_err());

    let migration_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("migration count should be readable");
    let partial_table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'partial_fixture'",
            [],
            |row| row.get(0),
        )
        .expect("table list should be readable");

    assert_eq!(migration_count, 0);
    assert_eq!(partial_table_count, 0);
}

#[test]
fn connections_enable_foreign_keys() {
    let (_directory, database) = test_database();
    let connection = database.open_connection().expect("database should open");
    let foreign_keys: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("foreign key pragma should be readable");

    assert_eq!(foreign_keys, 1);
}

#[test]
fn transaction_commits_work() {
    let (_directory, database) = test_database();
    database.initialize().expect("migration should succeed");
    let connection = database.open_connection().expect("database should open");
    connection
        .execute_batch("CREATE TABLE transaction_probe (value TEXT NOT NULL);")
        .expect("test-only table should be created");

    database
        .with_transaction(|transaction| {
            transaction
                .execute(
                    "INSERT INTO transaction_probe (value) VALUES ('committed')",
                    [],
                )
                .map_err(PersistenceError::DatabaseOperation)?;
            Ok(())
        })
        .expect("transaction should commit");

    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM transaction_probe", [], |row| {
            row.get(0)
        })
        .expect("probe count should be readable");
    assert_eq!(count, 1);
}

#[test]
fn transaction_rolls_back_on_failure() {
    let (_directory, database) = test_database();
    database.initialize().expect("migration should succeed");
    let connection = database.open_connection().expect("database should open");
    connection
        .execute_batch("CREATE TABLE transaction_probe (value TEXT NOT NULL);")
        .expect("test-only table should be created");

    let result = database.with_transaction(|transaction| {
        transaction
            .execute(
                "INSERT INTO transaction_probe (value) VALUES ('rolled back')",
                [],
            )
            .map_err(PersistenceError::DatabaseOperation)?;
        transaction
            .execute("INSERT INTO missing_probe (value) VALUES ('fail')", [])
            .map_err(PersistenceError::DatabaseOperation)?;
        Ok(())
    });
    assert!(result.is_err());

    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM transaction_probe", [], |row| {
            row.get(0)
        })
        .expect("probe count should be readable");
    assert_eq!(count, 0);
}

#[test]
fn app_metadata_has_unique_keys_and_supports_read_write() {
    let (_directory, database) = test_database();
    database.initialize().expect("migration should succeed");
    let connection = database.open_connection().expect("database should open");
    connection
        .execute(
            "INSERT INTO app_metadata (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params!["test.key", "value", 1_i64],
        )
        .expect("metadata should be written");
    let duplicate = connection.execute(
        "INSERT INTO app_metadata (key, value, updated_at) VALUES (?1, ?2, ?3)",
        params!["test.key", "other", 2_i64],
    );
    let value: String = connection
        .query_row(
            "SELECT value FROM app_metadata WHERE key = ?1",
            ["test.key"],
            |row| row.get(0),
        )
        .expect("metadata should be readable");

    assert!(duplicate.is_err());
    assert_eq!(value, "value");
}

fn failing_migration(transaction: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute_batch(
        "CREATE TABLE partial_fixture (value TEXT NOT NULL);
         INSERT INTO missing_fixture (value) VALUES ('fails');",
    )
}

fn seed_migration_history(connection: &rusqlite::Connection, versions: &[i64]) {
    connection
        .execute_batch(
            "CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at INTEGER NOT NULL
            );",
        )
        .expect("migration tracking fixture should be created");
    for version in versions {
        connection
            .execute(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                params![version, format!("fixture-{version}"), 0_i64],
            )
            .expect("migration history fixture should be inserted");
    }
}

fn test_migration_registry() -> [Migration; 3] {
    [
        Migration {
            version: 1,
            name: "001_fixture",
            apply: successful_fixture_migration,
        },
        Migration {
            version: 2,
            name: "002_fixture",
            apply: apply_migration_two_fixture,
        },
        Migration {
            version: 3,
            name: "003_fixture",
            apply: apply_migration_three_fixture,
        },
    ]
}

fn successful_fixture_migration(_: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    Ok(())
}

fn apply_migration_two_fixture(transaction: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute_batch("CREATE TABLE migration_two_applied (value TEXT NOT NULL);")
}

fn apply_migration_three_fixture(transaction: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    transaction.execute_batch("CREATE TABLE migration_three_applied (value TEXT NOT NULL);")
}

fn table_exists(connection: &rusqlite::Connection, table_name: &str) -> bool {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [table_name],
            |row| row.get(0),
        )
        .expect("table lookup should succeed")
}
