use std::{path::PathBuf, time::Duration};

use rusqlite::{Connection, Transaction};

use super::{run_migrations, PersistenceError};

/// A lightweight database boundary. Each use opens a configured short-lived connection.
#[derive(Debug, Clone)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn initialize(&self) -> Result<(), PersistenceError> {
        let mut connection = self.open_connection()?;
        run_migrations(&mut connection)?;
        Ok(())
    }

    pub fn open_connection(&self) -> Result<Connection, PersistenceError> {
        let connection = Connection::open(&self.path).map_err(PersistenceError::DatabaseOpen)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(PersistenceError::DatabaseOperation)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(PersistenceError::DatabaseOperation)?;
        Ok(connection)
    }

    #[allow(dead_code)] // Reserved for future application services; exercised by persistence tests.
    pub fn with_transaction<T, F>(&self, operation: F) -> Result<T, PersistenceError>
    where
        F: FnOnce(&Transaction<'_>) -> Result<T, PersistenceError>,
    {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(PersistenceError::Transaction)?;
        let value = operation(&transaction)?;
        transaction
            .commit()
            .map_err(PersistenceError::Transaction)?;
        Ok(value)
    }
}
