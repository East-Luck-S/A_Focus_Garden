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
fn empty_database_applies_migration_one() {
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

    assert_eq!(migration_count, 1);
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
