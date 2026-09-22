use std::{fs, path::PathBuf};

use serde::Serialize;

use crate::infrastructure::persistence::{Database, PersistenceError, CURRENT_SCHEMA_VERSION};

pub const DATABASE_FILE_NAME: &str = "focus-garden.sqlite3";

/// The only M1 application service for native persistence health information.
#[derive(Debug)]
pub struct PersistenceService {
    database: Database,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistenceStatus {
    pub initialized: bool,
    pub schema_version: i64,
    pub database_file_name: &'static str,
}

impl PersistenceService {
    pub fn initialize(app_local_data_dir: PathBuf) -> Result<Self, PersistenceError> {
        fs::create_dir_all(&app_local_data_dir).map_err(PersistenceError::Directory)?;
        let database = Database::new(app_local_data_dir.join(DATABASE_FILE_NAME));
        database.initialize()?;
        Ok(Self { database })
    }

    pub fn get_persistence_status(&self) -> Result<PersistenceStatus, PersistenceError> {
        let connection = self.database.open_connection()?;
        let schema_version = connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get::<_, Option<i64>>(0)
            })
            .map_err(PersistenceError::DatabaseOperation)?
            .unwrap_or_default();

        Ok(PersistenceStatus {
            initialized: schema_version == CURRENT_SCHEMA_VERSION,
            schema_version,
            database_file_name: DATABASE_FILE_NAME,
        })
    }
}
