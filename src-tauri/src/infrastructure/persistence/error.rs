use std::time::SystemTimeError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("unable to resolve the application local data directory")]
    AppDataDirectory,
    #[error("unable to start the native application")]
    NativeRuntime,
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
}

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("unable to create the persistence directory")]
    Directory(#[source] std::io::Error),
    #[error("unable to open the SQLite database")]
    DatabaseOpen(#[source] rusqlite::Error),
    #[error("SQLite operation failed")]
    DatabaseOperation(#[source] rusqlite::Error),
    #[allow(dead_code)] // Constructed by the transaction foundation, which is not yet a product use case.
    #[error("transaction operation failed")]
    Transaction(#[source] rusqlite::Error),
    #[error(transparent)]
    Migration(#[from] MigrationError),
    #[error("database schema version {database_version} is newer than supported version {current_version}")]
    UnsupportedFutureSchema {
        database_version: i64,
        current_version: i64,
    },
    #[error("migration history is not a continuous prefix starting at version 1")]
    MigrationHistoryInvalid,
    #[error("system clock is unavailable")]
    Clock(#[source] SystemTimeError),
}

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("unable to bootstrap migration tracking")]
    Bootstrap(#[source] rusqlite::Error),
    #[error("migration {version} ({name}) failed")]
    Failed {
        version: i64,
        name: &'static str,
        #[source]
        source: rusqlite::Error,
    },
}
