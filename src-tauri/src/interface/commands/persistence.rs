use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{
    application::persistence_service::{AppSettings, PersistenceService, PersistenceStatus},
    infrastructure::persistence::{AppError, PersistenceError},
};

#[derive(Debug, Serialize)]
pub struct NativeError {
    pub code: &'static str,
    pub message: &'static str,
}

impl From<AppError> for NativeError {
    fn from(error: AppError) -> Self {
        match error {
            AppError::AppDataDirectory => Self {
                code: "APP_DATA_DIRECTORY_UNAVAILABLE",
                message: "The application data directory is unavailable.",
            },
            AppError::NativeRuntime => Self {
                code: "NATIVE_RUNTIME_FAILURE",
                message: "The native application runtime is unavailable.",
            },
            AppError::Persistence(error) => Self::from(error),
        }
    }
}

impl From<PersistenceError> for NativeError {
    fn from(error: PersistenceError) -> Self {
        let (code, message) = match error {
            PersistenceError::InvalidFocusDuration => {
                ("INVALID_FOCUS_DURATION", "Choose a valid focus duration.")
            }
            PersistenceError::DataSafetyBlocker => (
                "DATA_SAFETY_BLOCKER",
                "Focus Garden stopped because its transactional journal is inconsistent.",
            ),
            PersistenceError::Directory(_) => (
                "PERSISTENCE_DIRECTORY_FAILURE",
                "The persistence directory could not be prepared.",
            ),
            PersistenceError::DatabaseOpen(_) => (
                "DATABASE_OPEN_FAILURE",
                "The local database could not be opened.",
            ),
            PersistenceError::DatabaseOperation(_) => (
                "DATABASE_OPERATION_FAILURE",
                "A local database operation failed.",
            ),
            PersistenceError::Transaction(_) => (
                "TRANSACTION_FAILURE",
                "A local database transaction failed.",
            ),
            PersistenceError::Migration(_) => (
                "MIGRATION_FAILED",
                "The local database migration could not be completed.",
            ),
            PersistenceError::UnsupportedFutureSchema { .. } => (
                "UNSUPPORTED_FUTURE_SCHEMA",
                "This database was created by a newer version of Focus Garden.",
            ),
            PersistenceError::MigrationHistoryInvalid => (
                "MIGRATION_HISTORY_INVALID",
                "The local database migration history is invalid.",
            ),
            PersistenceError::Clock(_) => (
                "CLOCK_UNAVAILABLE",
                "The system clock is unavailable for persistence initialization.",
            ),
        };
        Self { code, message }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAppSettingsRequest {
    pub default_focus_duration_ms: i64,
}

#[tauri::command]
pub fn get_app_settings(
    service: State<'_, PersistenceService>,
) -> Result<AppSettings, NativeError> {
    service.get_app_settings().map_err(NativeError::from)
}

#[tauri::command]
pub fn update_app_settings(
    request: UpdateAppSettingsRequest,
    service: State<'_, PersistenceService>,
) -> Result<AppSettings, NativeError> {
    service
        .update_app_settings(request.default_focus_duration_ms)
        .map_err(NativeError::from)
}

#[tauri::command]
pub fn get_persistence_status(
    service: State<'_, PersistenceService>,
) -> Result<PersistenceStatus, NativeError> {
    service.get_persistence_status().map_err(NativeError::from)
}

#[cfg(test)]
mod tests {
    use super::NativeError;
    use crate::{
        application::focus_session_service::FocusSessionServiceError,
        infrastructure::{
            focus_session_repository::SessionRepositoryError,
            persistence::{MigrationError, PersistenceError},
        },
    };

    const SENTINEL: &str = "C:\\private\\user.sqlite SELECT plant_seed";

    #[test]
    fn native_error_payloads_never_serialize_underlying_diagnostics() {
        let errors = [
            NativeError::from(PersistenceError::Directory(std::io::Error::other(SENTINEL))),
            NativeError::from(PersistenceError::DatabaseOpen(
                rusqlite::Error::InvalidParameterName(SENTINEL.to_owned()),
            )),
            NativeError::from(PersistenceError::Migration(MigrationError::Failed {
                version: 99,
                name: SENTINEL,
                source: rusqlite::Error::InvalidQuery,
            })),
            NativeError::from(FocusSessionServiceError::Repository(
                SessionRepositoryError::Database(rusqlite::Error::InvalidParameterName(
                    SENTINEL.to_owned(),
                )),
            )),
        ];

        for error in errors {
            let payload = serde_json::to_string(&error).expect("native error should serialize");
            assert!(!payload.contains(SENTINEL));
            assert!(!payload.contains("private"));
            assert!(!payload.contains("plant_seed"));
        }
    }
}
