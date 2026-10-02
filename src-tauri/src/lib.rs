//! W3wright World Editor: a desktop interface over the `w3wright` core crates.
//!
//! The interface holds no format knowledge. Everything it shows comes from
//! [`dto`] built out of a core parse, and everything it can do goes through
//! [`commands`] — see `docs/03` §1.1 for why that line is drawn here and not
//! somewhere more convenient.

mod commands;
mod dto;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::open_map,
            commands::preview_save,
            commands::read_terrain,
            commands::backend_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
