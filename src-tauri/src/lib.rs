mod application;
mod domain;
mod infrastructure;
mod interface;

use application::{
    focus_session_service::FocusSessionService, persistence_service::PersistenceService,
};
use infrastructure::persistence::AppError;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), AppError> {
    tauri::Builder::default()
        .setup(|app| {
            let app_local_data_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|_| AppError::AppDataDirectory)?;
            let service = PersistenceService::initialize(app_local_data_dir)?;
            let focus_session_service = FocusSessionService::new(service.database());
            focus_session_service.recover_active_sessions_on_launch()?;
            app.manage(service);
            app.manage(focus_session_service);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            interface::commands::persistence::get_persistence_status,
            interface::commands::persistence::get_app_settings,
            interface::commands::persistence::update_app_settings,
            interface::commands::focus_session::get_current_focus_session,
            interface::commands::focus_session::start_focus_session,
            interface::commands::focus_session::pause_focus_session,
            interface::commands::focus_session::resume_focus_session,
            interface::commands::focus_session::abandon_focus_session,
            interface::commands::focus_session::resolve_focus_session_recovery,
            interface::commands::focus_session::finalize_focus_session,
            interface::commands::focus_session::list_garden_plants,
            interface::commands::focus_session::get_garden_plant
        ])
        .run(tauri::generate_context!())
        .map_err(|_| AppError::NativeRuntime)
}
