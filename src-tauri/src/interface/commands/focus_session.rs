use serde::Deserialize;
use tauri::State;

use crate::{
    application::focus_session_service::{
        FocusSessionService, FocusSessionServiceError, FocusSessionSnapshot, GardenOverview,
        GardenPlantDetail, GardenPlantPage, ListGardenPlantsRequest,
    },
    domain::focus_session::RecoveryResolution,
    interface::commands::persistence::NativeError,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartFocusSessionRequest {
    pub planned_duration_ms: i64,
}

#[derive(Debug, Deserialize)]
pub struct FocusSessionIdRequest {
    pub id: String,
}

#[tauri::command]
pub fn finalize_focus_session(
    request: FocusSessionIdRequest,
    service: State<'_, FocusSessionService>,
) -> Result<GardenPlantDetail, NativeError> {
    service
        .finalize_focus_session(&request.id)
        .map_err(NativeError::from)
}

#[tauri::command]
pub fn list_garden_plants(
    request: ListGardenPlantsRequest,
    service: State<'_, FocusSessionService>,
) -> Result<GardenPlantPage, NativeError> {
    service
        .list_garden_plants_page(request)
        .map_err(NativeError::from)
}

#[tauri::command]
pub fn get_garden_overview(
    service: State<'_, FocusSessionService>,
) -> Result<GardenOverview, NativeError> {
    service.get_garden_overview().map_err(NativeError::from)
}

#[tauri::command]
pub fn get_garden_plant(
    request: FocusSessionIdRequest,
    service: State<'_, FocusSessionService>,
) -> Result<GardenPlantDetail, NativeError> {
    service
        .get_garden_plant(&request.id)
        .map_err(NativeError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRecoveryRequest {
    pub id: String,
    pub resolution: RecoveryResolution,
}

#[tauri::command]
pub fn get_current_focus_session(
    service: State<'_, FocusSessionService>,
) -> Result<Option<FocusSessionSnapshot>, NativeError> {
    service.get_current().map_err(NativeError::from)
}

#[tauri::command]
pub fn start_focus_session(
    request: StartFocusSessionRequest,
    service: State<'_, FocusSessionService>,
) -> Result<FocusSessionSnapshot, NativeError> {
    service
        .start(request.planned_duration_ms)
        .map_err(NativeError::from)
}

#[tauri::command]
pub fn pause_focus_session(
    request: FocusSessionIdRequest,
    service: State<'_, FocusSessionService>,
) -> Result<FocusSessionSnapshot, NativeError> {
    service.pause(&request.id).map_err(NativeError::from)
}

#[tauri::command]
pub fn resume_focus_session(
    request: FocusSessionIdRequest,
    service: State<'_, FocusSessionService>,
) -> Result<FocusSessionSnapshot, NativeError> {
    service.resume(&request.id).map_err(NativeError::from)
}

#[tauri::command]
pub fn abandon_focus_session(
    request: FocusSessionIdRequest,
    service: State<'_, FocusSessionService>,
) -> Result<FocusSessionSnapshot, NativeError> {
    service.abandon(&request.id).map_err(NativeError::from)
}

#[tauri::command]
pub fn resolve_focus_session_recovery(
    request: ResolveRecoveryRequest,
    service: State<'_, FocusSessionService>,
) -> Result<FocusSessionSnapshot, NativeError> {
    service
        .resolve_recovery(&request.id, request.resolution)
        .map_err(NativeError::from)
}

impl From<FocusSessionServiceError> for NativeError {
    fn from(error: FocusSessionServiceError) -> Self {
        use crate::{
            domain::focus_session::SessionError,
            infrastructure::focus_session_repository::SessionRepositoryError,
        };
        match error {
            FocusSessionServiceError::GardenQueryInvalid => Self {
                code: "GARDEN_QUERY_INVALID",
                message: "The garden page request is invalid.",
            },
            FocusSessionServiceError::Domain(SessionError::InvalidDuration) => Self {
                code: "INVALID_FOCUS_DURATION",
                message: "Choose a valid focus duration.",
            },
            FocusSessionServiceError::Domain(SessionError::InvalidTransition) => Self {
                code: "SESSION_INVALID_TRANSITION",
                message: "That session action is not valid right now.",
            },
            FocusSessionServiceError::Domain(SessionError::NotReadyToComplete) => Self {
                code: "SESSION_NOT_READY_TO_COMPLETE",
                message: "The focus session has not reached its end time.",
            },
            FocusSessionServiceError::Domain(SessionError::DataInvalid)
            | FocusSessionServiceError::Repository(SessionRepositoryError::DataInvalid) => Self {
                code: "SESSION_DATA_INVALID",
                message: "The stored focus session data is invalid.",
            },
            FocusSessionServiceError::GenerationDataInvalid => Self {
                code: "SESSION_GENERATION_DATA_INVALID",
                message: "The stored plant preview for this focus session is invalid.",
            },
            FocusSessionServiceError::DataSafetyBlocker => Self {
                code: "DATA_SAFETY_BLOCKER",
                message: "Focus Garden stopped because its transactional journal is inconsistent.",
            },
            FocusSessionServiceError::PlantNotFound => Self {
                code: "PLANT_NOT_FOUND",
                message: "The requested garden plant was not found.",
            },
            FocusSessionServiceError::Domain(SessionError::TimeValueInvalid)
            | FocusSessionServiceError::TimeValueInvalid => Self {
                code: "TIME_VALUE_INVALID",
                message: "A time value for this focus session is invalid.",
            },
            FocusSessionServiceError::Repository(SessionRepositoryError::OpenSessionExists) => {
                Self {
                    code: "OPEN_SESSION_EXISTS",
                    message: "Finish or abandon the current focus session first.",
                }
            }
            FocusSessionServiceError::Repository(SessionRepositoryError::NotFound) => Self {
                code: "SESSION_NOT_FOUND",
                message: "The requested focus session no longer exists.",
            },
            FocusSessionServiceError::Repository(SessionRepositoryError::Database(_)) => Self {
                code: "DATABASE_OPERATION_FAILURE",
                message: "A local database operation failed.",
            },
            FocusSessionServiceError::Persistence(error) => Self::from(error),
        }
    }
}
