//! Comandos Tauri do histórico local (EP06) sobre `texlocal-git`.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager, State};
use texlocal_git as git;
use texlocal_git::{
    CommitInfo, FileDiff, FileSides, Identity, LogQuery, RepoStatus, RestoreResult, TagInfo,
};
use texlocal_tex::compile::{self, CompileRequest, CompileStatus};
use texlocal_tex::manifest;

/// Serializa as operações que escrevem no repositório (evita disputa pelo `index.lock`).
#[derive(Default)]
pub struct HistoryState {
    write: Arc<Mutex<()>>,
}

/// Prefixo do erro quando falta a identidade; a UI usa para abrir o diálogo de identificação.
const IDENTITY_REQUIRED: &str = "identity-required";

fn err(e: git::Error) -> String {
    match e {
        git::Error::Busy(m) => format!("busy: {m}"),
        e => e.to_string(),
    }
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

fn identity_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|d| d.join("git-identity.json"))
        .map_err(|e| e.to_string())
}

fn read_identity(app: &AppHandle) -> Option<Identity> {
    let text = std::fs::read_to_string(identity_path(app).ok()?).ok()?;
    let id: Identity = serde_json::from_str(&text).ok()?;
    (!id.name.trim().is_empty() && !id.email.trim().is_empty()).then_some(id)
}

fn require_identity(app: &AppHandle) -> Result<Identity, String> {
    read_identity(app).ok_or_else(|| IDENTITY_REQUIRED.to_owned())
}

/// Como `blocking`, mas exclusivo: só uma escrita no repositório por vez.
async fn write<T, F>(state: &State<'_, HistoryState>, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    let gate = Arc::clone(&state.write);
    blocking(move || {
        let _g = gate.lock().unwrap_or_else(|e| e.into_inner());
        f()
    })
    .await
}

// ───────────────────────── identidade ─────────────────────────

#[tauri::command]
pub fn history_identity_get(app: AppHandle) -> Option<Identity> {
    read_identity(&app)
}

/// Salva nome/e-mail na config do app (nunca altera o `~/.gitconfig`).
#[tauri::command]
pub fn history_identity_set(
    app: AppHandle,
    name: String,
    email: String,
) -> Result<Identity, String> {
    let id = Identity {
        name: name.trim().to_owned(),
        email: email.trim().to_owned(),
    };
    if id.name.is_empty() || !id.email.contains('@') {
        return Err("informe um nome e um e-mail válidos".into());
    }
    let path = identity_path(&app)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&id).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(id)
}

// ───────────────────────── repositório ─────────────────────────

#[tauri::command]
pub async fn history_status(root: String) -> Result<RepoStatus, String> {
    blocking(move || git::status(Path::new(&root)).map_err(err)).await
}

#[tauri::command]
pub async fn history_init(
    app: AppHandle,
    state: State<'_, HistoryState>,
    root: String,
) -> Result<Option<CommitInfo>, String> {
    let id = require_identity(&app)?;
    write(&state, move || {
        git::init(Path::new(&root), &id).map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn history_checkpoint(
    app: AppHandle,
    state: State<'_, HistoryState>,
    root: String,
    message: Option<String>,
) -> Result<Option<CommitInfo>, String> {
    let id = require_identity(&app)?;
    write(&state, move || {
        git::checkpoint(Path::new(&root), &id, message.as_deref()).map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn history_commit(
    app: AppHandle,
    state: State<'_, HistoryState>,
    root: String,
    message: String,
    amend: bool,
) -> Result<Option<CommitInfo>, String> {
    let id = require_identity(&app)?;
    write(&state, move || {
        git::commit_staged(Path::new(&root), &id, &message, amend).map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn history_stage(
    state: State<'_, HistoryState>,
    root: String,
    paths: Vec<String>,
    staged: bool,
) -> Result<(), String> {
    write(&state, move || {
        let root = Path::new(&root);
        if staged {
            git::stage(root, &paths)
        } else {
            git::unstage(root, &paths)
        }
        .map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn history_log(root: String, query: LogQuery) -> Result<Vec<CommitInfo>, String> {
    blocking(move || git::log(Path::new(&root), &query).map_err(err)).await
}

#[tauri::command]
pub async fn history_changed(
    root: String,
    from: Option<String>,
    to: Option<String>,
) -> Result<Vec<FileDiff>, String> {
    blocking(move || {
        git::changed_files(Path::new(&root), from.as_deref(), to.as_deref()).map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn history_sides(
    root: String,
    from: Option<String>,
    to: Option<String>,
    path: String,
) -> Result<FileSides, String> {
    blocking(move || {
        git::file_sides(Path::new(&root), from.as_deref(), to.as_deref(), &path).map_err(err)
    })
    .await
}

// ───────────────────────── versões nomeadas ─────────────────────────

#[tauri::command]
pub async fn history_tags(root: String) -> Result<Vec<TagInfo>, String> {
    blocking(move || git::tag_list(Path::new(&root)).map_err(err)).await
}

/// Nomeia um commit (`target`) ou, sem `target`, a versão atual (com checkpoint se houver mudanças).
#[tauri::command]
pub async fn history_tag_create(
    app: AppHandle,
    state: State<'_, HistoryState>,
    root: String,
    target: Option<String>,
    label: String,
    description: String,
) -> Result<TagInfo, String> {
    let id = require_identity(&app)?;
    write(&state, move || {
        let root = Path::new(&root);
        match target {
            Some(t) => git::tag_create(root, &id, Some(&t), &label, &description),
            None => git::tag_current(root, &id, &label, &description),
        }
        .map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn history_tag_rename(
    app: AppHandle,
    state: State<'_, HistoryState>,
    root: String,
    name: String,
    label: String,
    description: Option<String>,
) -> Result<TagInfo, String> {
    let id = require_identity(&app)?;
    write(&state, move || {
        git::tag_rename(Path::new(&root), &id, &name, &label, description.as_deref()).map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn history_tag_delete(
    state: State<'_, HistoryState>,
    root: String,
    name: String,
) -> Result<(), String> {
    write(&state, move || {
        git::tag_delete(Path::new(&root), &name).map_err(err)
    })
    .await
}

// ───────────────────────── restaurar e exportar ─────────────────────────

#[tauri::command]
pub async fn history_restore(
    app: AppHandle,
    state: State<'_, HistoryState>,
    root: String,
    target: String,
    paths: Option<Vec<String>>,
) -> Result<RestoreResult, String> {
    let id = require_identity(&app)?;
    write(&state, move || {
        git::restore(Path::new(&root), &id, &target, paths.as_deref()).map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn history_restore_text(
    app: AppHandle,
    state: State<'_, HistoryState>,
    root: String,
    source: String,
    path: String,
    text: String,
) -> Result<RestoreResult, String> {
    let id = require_identity(&app)?;
    write(&state, move || {
        git::restore_text(Path::new(&root), &id, &source, &path, &text).map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn history_export_zip(root: String, rev: String, dest: String) -> Result<usize, String> {
    blocking(move || git::export_zip(Path::new(&root), &rev, Path::new(&dest)).map_err(err)).await
}

/// Hash FNV-1a (estável entre execuções) para nomear o cache por projeto.
fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// Compila a versão `rev` numa pasta temporária isolada e copia o PDF para `dest`.
/// O PDF fica em cache por hash de commit.
#[tauri::command]
pub async fn history_export_pdf(
    app: AppHandle,
    root: String,
    rev: String,
    dest: String,
    bin_dir: Option<String>,
) -> Result<(), String> {
    let cache_root = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("history-pdf")
        .join(fnv(&root));
    blocking(move || {
        let root_path = Path::new(&root);
        let id = git::resolve(root_path, &rev).map_err(err)?;
        let cached = cache_root.join(format!("{id}.pdf"));
        if !cached.is_file() {
            let work = tempdir_in(&cache_root, &id)?;
            let result = (|| {
                git::export_tree(root_path, &id, &work).map_err(err)?;
                let manifest = manifest::read_or_create(&work).map_err(|e| e.to_string())?;
                let outcome = compile::compile(
                    &CompileRequest {
                        project_dir: work.clone(),
                        bin_dir: bin_dir.filter(|b| !b.trim().is_empty()).map(PathBuf::from),
                        manifest,
                        force: true,
                    },
                    &AtomicBool::new(false),
                )
                .map_err(|e| format!("{e:?}"))?;
                match (outcome.status, outcome.pdf) {
                    (CompileStatus::Success, Some(pdf)) => {
                        std::fs::create_dir_all(&cache_root).map_err(|e| e.to_string())?;
                        std::fs::copy(pdf, &cached).map_err(|e| e.to_string())?;
                        Ok(())
                    }
                    _ => Err(
                        "a compilação desta versão falhou; não foi possível gerar o PDF".to_owned(),
                    ),
                }
            })();
            let _ = std::fs::remove_dir_all(&work);
            result?;
        }
        std::fs::copy(&cached, &dest).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await
}

fn tempdir_in(cache_root: &Path, id: &str) -> Result<PathBuf, String> {
    let dir = cache_root.join(format!("work-{id}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}
