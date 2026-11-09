use std::fs;
use std::path::PathBuf;

use tauri::Manager;
use texlocal_core::settings::Settings;

#[tauri::command]
fn app_version() -> &'static str {
    texlocal_core::VERSION
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join("settings.json"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let path = settings_path(&app)?;
    match fs::read_to_string(&path) {
        Ok(json) => Settings::from_json(&json).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    settings.validate().map_err(|e| e.to_string())?;
    let path = settings_path(&app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&path, settings.to_json()).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_version,
            get_settings,
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar o TexLocal");
}
