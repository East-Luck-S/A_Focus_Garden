mod application;
mod domain;
mod infrastructure;
mod interface;

use application::{
    focus_session_service::FocusSessionService, persistence_service::PersistenceService,
};
use infrastructure::persistence::AppError;
use tauri::Manager;

#[cfg(windows)]
const WINDOW_CAPTION_COLOR: u32 = colorref(0xd8, 0xdd, 0xd1);
#[cfg(windows)]
const WINDOW_TEXT_COLOR: u32 = colorref(0x37, 0x48, 0x3f);

const fn colorref(red: u8, green: u8, blue: u8) -> u32 {
    red as u32 | (green as u32) << 8 | (blue as u32) << 16
}

#[cfg(windows)]
fn apply_windows_caption_colors(window: &tauri::WebviewWindow) {
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
    };

    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    for (attribute, color) in [
        (DWMWA_CAPTION_COLOR, WINDOW_CAPTION_COLOR),
        (DWMWA_TEXT_COLOR, WINDOW_TEXT_COLOR),
        (DWMWA_BORDER_COLOR, WINDOW_CAPTION_COLOR),
    ] {
        // DWM appearance is optional; an unsupported Windows version must not block startup.
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd.0,
                attribute as u32,
                &color as *const u32 as *const core::ffi::c_void,
                size_of::<u32>() as u32,
            );
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), AppError> {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(windows)]
            if let Some(window) = app.get_webview_window("main") {
                apply_windows_caption_colors(&window);
            }
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
            interface::commands::focus_session::get_garden_overview,
            interface::commands::focus_session::get_garden_plant
        ])
        .run(tauri::generate_context!())
        .map_err(|_| AppError::NativeRuntime)
}

#[cfg(test)]
mod tests {
    use super::colorref;

    #[test]
    fn colorref_uses_windows_bgr_byte_order() {
        assert_eq!(colorref(0xd8, 0xdd, 0xd1), 0x00d1ddd8);
        assert_eq!(colorref(0x37, 0x48, 0x3f), 0x003f4837);
    }
}
