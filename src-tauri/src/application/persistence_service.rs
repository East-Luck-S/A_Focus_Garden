use std::{fs, path::PathBuf};

use rusqlite::params;
use serde::Serialize;

use crate::{
    domain::focus_session::{MAXIMUM_FOCUS_DURATION_MS, MINIMUM_FOCUS_DURATION_MS},
    infrastructure::persistence::{Database, PersistenceError, CURRENT_SCHEMA_VERSION},
};

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

#[allow(dead_code)]
pub const DEFAULT_FOCUS_DURATION_MS: i64 = 25 * 60_000;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub default_focus_duration_ms: i64,
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

    pub fn database(&self) -> Database {
        self.database.clone()
    }

    pub fn get_app_settings(&self) -> Result<AppSettings, PersistenceError> {
        let connection = self.database.open_connection()?;
        let duration = connection
            .query_row(
                "SELECT default_focus_duration_ms FROM app_settings WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .map_err(PersistenceError::DatabaseOperation)?;
        if !(MINIMUM_FOCUS_DURATION_MS..=MAXIMUM_FOCUS_DURATION_MS).contains(&duration) {
            return Err(PersistenceError::DataSafetyBlocker);
        }
        Ok(AppSettings {
            default_focus_duration_ms: duration,
        })
    }

    pub fn update_app_settings(
        &self,
        default_focus_duration_ms: i64,
    ) -> Result<AppSettings, PersistenceError> {
        if !(MINIMUM_FOCUS_DURATION_MS..=MAXIMUM_FOCUS_DURATION_MS)
            .contains(&default_focus_duration_ms)
        {
            return Err(PersistenceError::InvalidFocusDuration);
        }
        let connection = self.database.open_connection()?;
        let updated = connection
            .execute(
                "UPDATE app_settings SET default_focus_duration_ms = ?1 WHERE singleton = 1",
                params![default_focus_duration_ms],
            )
            .map_err(PersistenceError::DatabaseOperation)?;
        if updated != 1 {
            return Err(PersistenceError::DataSafetyBlocker);
        }
        Ok(AppSettings {
            default_focus_duration_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn settings_default_boundaries_rejections_and_restart_are_persistent() {
        let directory = tempdir().unwrap();
        let service = PersistenceService::initialize(directory.path().to_path_buf()).unwrap();
        assert_eq!(
            service
                .get_app_settings()
                .unwrap()
                .default_focus_duration_ms,
            DEFAULT_FOCUS_DURATION_MS
        );
        for duration in [MINIMUM_FOCUS_DURATION_MS, MAXIMUM_FOCUS_DURATION_MS] {
            assert_eq!(
                service
                    .update_app_settings(duration)
                    .unwrap()
                    .default_focus_duration_ms,
                duration
            );
        }
        assert!(matches!(
            service.update_app_settings(0),
            Err(PersistenceError::InvalidFocusDuration)
        ));
        assert!(matches!(
            service.update_app_settings(MAXIMUM_FOCUS_DURATION_MS + 60_000),
            Err(PersistenceError::InvalidFocusDuration)
        ));
        assert_eq!(
            service
                .get_app_settings()
                .unwrap()
                .default_focus_duration_ms,
            MAXIMUM_FOCUS_DURATION_MS
        );
        let restarted = PersistenceService::initialize(directory.path().to_path_buf()).unwrap();
        assert_eq!(
            restarted
                .get_app_settings()
                .unwrap()
                .default_focus_duration_ms,
            MAXIMUM_FOCUS_DURATION_MS
        );
    }
}
