mod application;
mod domain;
mod infrastructure;
mod interface;

use application::persistence_service::PersistenceService;
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
            app.manage(service);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            interface::commands::persistence::get_persistence_status
        ])
        .run(tauri::generate_context!())
        .map_err(|_| AppError::NativeRuntime)
}
