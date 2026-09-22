// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = focus_garden_lib::run() {
        eprintln!("Focus Garden could not start: {error}");
        std::process::exit(1);
    }
}
