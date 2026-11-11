mod compile_commands;
mod editor_commands;
mod snippet_commands;
mod tex_commands;

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

/// Caminho do `settings.json` (criado com os padrões se ainda não existir), para abrir no editor.
#[tauri::command]
fn settings_file(app: tauri::AppHandle) -> Result<String, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        save_settings(app, Settings::default())?;
    }
    Ok(path.to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(tex_commands::InstallState::default())
        .manage(tex_commands::PackageState::default())
        .manage(editor_commands::ProjectState::default())
        .manage(compile_commands::CompileState::default())
        .manage(snippet_commands::SnippetWatchState::default())
        .invoke_handler(tauri::generate_handler![
            app_version,
            get_settings,
            save_settings,
            settings_file,
            tex_commands::detect_tex,
            tex_commands::tex_profiles,
            tex_commands::tex_disk_info,
            tex_commands::project_manifest,
            tex_commands::save_project_manifest,
            tex_commands::tex_install,
            tex_commands::tex_install_cancel,
            tex_commands::tex_resolve_package,
            tex_commands::tex_install_packages,
            tex_commands::tex_install_packages_cancel,
            tex_commands::resources_list,
            tex_commands::resources_remove,
            compile_commands::compile_start,
            compile_commands::compile_cancel,
            compile_commands::pdf_read,
            compile_commands::synctex_forward,
            compile_commands::synctex_inverse,
            editor_commands::file_read,
            editor_commands::file_write,
            editor_commands::project_files,
            editor_commands::path_exists,
            editor_commands::file_read_bytes,
            editor_commands::reveal_path,
            editor_commands::project_open,
            editor_commands::index_update,
            editor_commands::index_remove,
            editor_commands::project_search,
            editor_commands::project_replace,
            editor_commands::word_count_text,
            editor_commands::word_count_files,
            editor_commands::spell_configure,
            editor_commands::spell_check,
            editor_commands::spell_suggest,
            editor_commands::spell_detect_language,
            editor_commands::spell_add_word,
            snippet_commands::snippets_load,
            snippet_commands::snippets_watch
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar o TexLocal");
}
