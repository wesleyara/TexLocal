//! Comandos Tauri do gerenciador TeX (EP02).

use std::path::PathBuf;

use serde::Serialize;
use texlocal_tex::detect::{self, Distribution};
use texlocal_tex::manifest::{self, Manifest};
use texlocal_tex::profile::Profile;
use texlocal_tex::{disk, paths};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInfo {
    id: Profile,
    approx_size_mb: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    /// Local padrão da instalação gerenciada.
    path: String,
    free_bytes: Option<u64>,
}

async fn blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn detect_tex() -> Result<Vec<Distribution>, String> {
    blocking(|| {
        let path_env = std::env::var("PATH").unwrap_or_default();
        let extra: Vec<PathBuf> = paths::data_root()
            .map(|root| {
                let tex = root.join("texlive");
                std::fs::read_dir(tex)
                    .map(|years| {
                        years
                            .flatten()
                            .map(|y| paths::managed_bin_dir(&y.path()))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        Ok(detect::detect(&detect::candidate_bin_dirs(
            &path_env, &extra,
        )))
    })
    .await
}

#[tauri::command]
pub fn tex_profiles() -> Vec<ProfileInfo> {
    Profile::ALL
        .iter()
        .map(|p| ProfileInfo {
            id: *p,
            approx_size_mb: p.approx_size_mb(),
        })
        .collect()
}

#[tauri::command]
pub async fn tex_disk_info() -> Result<DiskInfo, String> {
    blocking(|| {
        let root = paths::data_root().ok_or("diretório de dados indisponível")?;
        Ok(DiskInfo {
            path: root.join("texlive").to_string_lossy().into_owned(),
            free_bytes: disk::free_space(&root),
        })
    })
    .await
}

#[tauri::command]
pub async fn project_manifest(dir: String) -> Result<Manifest, String> {
    blocking(move || manifest::read_or_create(&PathBuf::from(dir))).await
}

#[tauri::command]
pub async fn save_project_manifest(dir: String, manifest: Manifest) -> Result<(), String> {
    blocking(move || manifest::write(&PathBuf::from(dir), &manifest)).await
}
