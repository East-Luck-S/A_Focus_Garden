// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(any(test, all(target_os = "windows", not(debug_assertions))))]
fn utf16_nul(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(any(test, all(target_os = "windows", not(debug_assertions))))]
fn startup_message(error: &str) -> &'static str {
    if [
        "UNSUPPORTED_FUTURE_SCHEMA",
        "MIGRATION_HISTORY_INVALID",
        "DATA_SAFETY_BLOCKER",
    ]
    .iter()
    .any(|code| error.contains(code))
    {
        "为保护本机数据，专注花园已停止启动，不会自动重置数据。请保留当前数据并联系支持人员。"
    } else {
        "专注花园暂时无法启动。请稍后重试；你的本机数据不会被自动重置。"
    }
}

#[cfg(all(target_os = "windows", not(debug_assertions)))]
fn show_startup_error_message(error: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONERROR, MB_OK, MB_SETFOREGROUND,
    };

    let title = utf16_nul("专注花园");
    let message = utf16_nul(startup_message(error));

    // This is deliberately a stable, non-diagnostic release message. Detailed errors remain on stderr.
    unsafe {
        MessageBoxW(
            core::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

fn main() {
    if let Err(error) = focus_garden_lib::run() {
        eprintln!("专注花园无法启动: {error}");
        #[cfg(all(target_os = "windows", not(debug_assertions)))]
        show_startup_error_message(&error.to_string());
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{startup_message, utf16_nul};

    #[test]
    fn utf16_helper_appends_exactly_one_nul_terminator() {
        let value = utf16_nul("专注花园");
        assert_eq!(value.last(), Some(&0));
        assert_eq!(
            String::from_utf16(&value[..value.len() - 1]).expect("valid UTF-16"),
            "专注花园"
        );
    }

    #[test]
    fn startup_message_keeps_data_safety_failures_generic_and_protective() {
        assert!(startup_message("DATA_SAFETY_BLOCKER: C:\\private").contains("不会自动重置"));
        assert!(startup_message("UNSUPPORTED_FUTURE_SCHEMA").contains("保护本机数据"));
        assert!(!startup_message("DATABASE_OPERATION_FAILURE: C:\\private").contains("private"));
    }
}
