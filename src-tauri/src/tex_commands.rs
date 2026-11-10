//! Comandos Tauri do gerenciador TeX (EP02).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use texlocal_tex::detect::{self, Distribution};
use texlocal_tex::installer::{InstallError, InstallErrorKind, InstallResult, Installer};
use texlocal_tex::logparse::{choose_package, parse_search_output, PackageIndex};
use texlocal_tex::manifest::{self, Manifest};
use texlocal_tex::profile::Profile;
use texlocal_tex::resources::{self, Resource};
use texlocal_tex::tlmgr::{Tlmgr, TlmgrError, TlmgrOp};
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

/// Estado da instalação gerenciada (no máximo uma por vez).
#[derive(Default)]
pub struct InstallState {
    cancel: Arc<AtomicBool>,
    running: AtomicBool,
}

/// Instala o TeX Live gerenciado, emitindo o evento `tex-install` com o progresso.
#[tauri::command]
pub async fn tex_install(
    app: AppHandle,
    state: State<'_, InstallState>,
    profile: Profile,
    mirror: Option<String>,
) -> Result<InstallResult, InstallError> {
    if state.running.swap(true, Ordering::SeqCst) {
        return Err(InstallError::new(
            InstallErrorKind::InstallFailed,
            "instalação já em andamento",
        ));
    }
    state.cancel.store(false, Ordering::SeqCst);
    let cancel = Arc::clone(&state.cancel);

    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let root = paths::data_root().ok_or_else(|| {
            InstallError::new(InstallErrorKind::Io, "diretório de dados indisponível")
        })?;
        let mirror = mirror.filter(|m| !m.trim().is_empty());
        Installer::new(root, mirror).install(profile, &cancel, &mut |event| {
            let _ = app.emit("tex-install", event);
        })
    })
    .await;
    state.running.store(false, Ordering::SeqCst);
    outcome.unwrap_or_else(|e| {
        Err(InstallError::new(
            InstallErrorKind::InstallFailed,
            e.to_string(),
        ))
    })
}

#[tauri::command]
pub fn tex_install_cancel(state: State<'_, InstallState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

/// Recursos baixados pelo app (instalações do TeX Live e cache), com tamanho em disco.
#[tauri::command]
pub async fn resources_list() -> Result<Vec<Resource>, String> {
    blocking(|| {
        let root = paths::data_root().ok_or("diretório de dados indisponível")?;
        Ok(resources::list(&root))
    })
    .await
}

/// Remove um recurso listado por `resources_list`. Recusa enquanto uma instalação roda.
#[tauri::command]
pub async fn resources_remove(state: State<'_, InstallState>, id: String) -> Result<(), String> {
    if state.running.load(Ordering::SeqCst) {
        return Err("instalação em andamento".into());
    }
    blocking(move || {
        let root = paths::data_root().ok_or("diretório de dados indisponível")?;
        resources::remove(&root, &id).map(|_| ())
    })
    .await
}

// ───────────────────────── pacotes faltantes (TL-208 / TL-407) ─────────────────────────

/// Cancelamento da instalação de pacotes em andamento.
#[derive(Default)]
pub struct PackageState {
    cancel: Arc<AtomicBool>,
}

fn locate_tlmgr(bin_dir: Option<&str>) -> Result<PathBuf, TlmgrError> {
    let dir_hint = bin_dir.filter(|d| !d.trim().is_empty());
    let found = match dir_hint {
        Some(dir) => detect::find_executable(std::path::Path::new(dir), "tlmgr"),
        None => std::env::var_os("PATH").and_then(|path| {
            std::env::split_paths(&path).find_map(|d| detect::find_executable(&d, "tlmgr"))
        }),
    };
    found.ok_or_else(|| TlmgrError::unavailable(dir_hint.unwrap_or("PATH").to_owned()))
}

fn index_path() -> Option<PathBuf> {
    paths::data_root().map(|r| r.join("package-index.json"))
}

/// Descobre o pacote do TeX Live que contém `file` (ex.: `abntex2.cls` -> `abntex2`).
/// Usa o cache local e só consulta o `tlmgr` (rede) quando preciso.
#[tauri::command]
pub async fn tex_resolve_package(
    bin_dir: Option<String>,
    usermode: bool,
    file: String,
) -> Result<Option<String>, TlmgrError> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = index_path();
        let mut index = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|j| PackageIndex::from_json(&j))
            .unwrap_or_default();
        if let Some(hit) = index.get(&file) {
            return Ok(Some(hit.to_owned()));
        }
        let tlmgr = Tlmgr::new(locate_tlmgr(bin_dir.as_deref())?, usermode);
        let out = tlmgr.run(
            &TlmgrOp::SearchFile(file.clone()),
            &AtomicBool::new(false),
            &mut |_| {},
        )?;
        let candidates = parse_search_output(&out, &file);
        let chosen = choose_package(&candidates).cloned();
        if let (Some(pkg), Some(p)) = (&chosen, &path) {
            index.insert(&file, pkg);
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(p, index.to_json());
        }
        Ok(chosen)
    })
    .await
    .map_err(|e| TlmgrError::unavailable(e.to_string()))?
}

/// Instala pacotes com o `tlmgr`, emitindo `tlmgr-progress` a cada pacote.
#[tauri::command]
pub async fn tex_install_packages(
    app: AppHandle,
    state: State<'_, PackageState>,
    bin_dir: Option<String>,
    usermode: bool,
    packages: Vec<String>,
) -> Result<(), TlmgrError> {
    state.cancel.store(false, Ordering::SeqCst);
    let cancel = Arc::clone(&state.cancel);
    tauri::async_runtime::spawn_blocking(move || {
        let tlmgr = Tlmgr::new(locate_tlmgr(bin_dir.as_deref())?, usermode);
        tlmgr
            .run(&TlmgrOp::Install(packages), &cancel, &mut |p| {
                let _ = app.emit("tlmgr-progress", p);
            })
            .map(|_| ())
    })
    .await
    .map_err(|e| TlmgrError::unavailable(e.to_string()))?
}

#[tauri::command]
pub fn tex_install_packages_cancel(state: State<'_, PackageState>) {
    state.cancel.store(true, Ordering::SeqCst);
}
