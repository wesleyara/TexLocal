//! Comandos Tauri de compilação, PDF e SyncTeX (EP04).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use tauri::ipc::Response;
use tauri::{AppHandle, Manager, State};
use texlocal_tex::compile::{self, CompileError, CompileErrorKind, CompileOutcome, CompileRequest};
use texlocal_tex::manifest::{self, Manifest};
use texlocal_tex::synctex::{ForwardResult, SourcePosition, SyncIndex};

/// Compilações em andamento (no máximo uma por projeto) e o índice SyncTeX em cache.
#[derive(Default)]
pub struct CompileState {
    running: Mutex<HashMap<String, Arc<AtomicBool>>>,
    sync: Mutex<Option<CachedSync>>,
}

struct CachedSync {
    path: PathBuf,
    modified: Option<SystemTime>,
    len: u64,
    index: Arc<SyncIndex>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Remove o projeto da lista de compilações ao terminar (inclusive em caso de pânico).
struct RunningGuard<'a> {
    state: &'a CompileState,
    key: String,
}

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        lock(&self.state.running).remove(&self.key);
    }
}

fn read_manifest(dir: &Path) -> Result<Manifest, CompileError> {
    manifest::read_or_create(dir).map_err(|e| CompileError::new(CompileErrorKind::Manifest, e))
}

/// Compila o projeto em `dir`. `bin_dir` é o `bin` da distribuição TeX escolhida (ou `None`
/// para usar o PATH).
#[tauri::command]
pub async fn compile_start(
    state: State<'_, CompileState>,
    dir: String,
    bin_dir: Option<String>,
) -> Result<CompileOutcome, CompileError> {
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut running = lock(&state.running);
        if running.contains_key(&dir) {
            return Err(CompileError::new(
                CompileErrorKind::Busy,
                "compilação já em andamento",
            ));
        }
        running.insert(dir.clone(), Arc::clone(&cancel));
    }
    let _guard = RunningGuard {
        state: &state,
        key: dir.clone(),
    };

    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let project_dir = PathBuf::from(&dir);
        let manifest = read_manifest(&project_dir)?;
        compile::compile(
            &CompileRequest {
                project_dir,
                bin_dir: bin_dir.filter(|b| !b.trim().is_empty()).map(PathBuf::from),
                manifest,
            },
            &cancel,
        )
    })
    .await;
    outcome.unwrap_or_else(|e| Err(CompileError::new(CompileErrorKind::Io, e.to_string())))
}

#[tauri::command]
pub fn compile_cancel(state: State<'_, CompileState>, dir: String) {
    if let Some(flag) = lock(&state.running).get(&dir) {
        flag.store(true, Ordering::SeqCst);
    }
}

/// Bytes do PDF, sem passar por JSON (o PDF.js recebe um `ArrayBuffer`).
#[tauri::command]
pub async fn pdf_read(path: String) -> Result<Response, String> {
    if !path.to_lowercase().ends_with(".pdf") {
        return Err("apenas arquivos .pdf".into());
    }
    let bytes = tauri::async_runtime::spawn_blocking(move || std::fs::read(&path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(Response::new(bytes))
}

/// Caminho do `.synctex.gz` do documento principal.
fn synctex_path(dir: &Path, manifest: &Manifest) -> PathBuf {
    let stem = Path::new(&manifest.project.main)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "main".into());
    dir.join(&manifest.compile.outdir)
        .join(format!("{stem}.synctex.gz"))
}

fn load_index(state: &CompileState, dir: &Path) -> Result<Arc<SyncIndex>, String> {
    let manifest = read_manifest(dir).map_err(|e| e.to_string())?;
    let path = synctex_path(dir, &manifest);
    let meta = std::fs::metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let modified = meta.modified().ok();

    let mut cache = lock(&state.sync);
    if let Some(c) = cache.as_ref() {
        if c.path == path && c.modified == modified && c.len == meta.len() {
            return Ok(Arc::clone(&c.index));
        }
    }
    let index = Arc::new(SyncIndex::load(&path).map_err(|e| e.to_string())?);
    *cache = Some(CachedSync {
        path,
        modified,
        len: meta.len(),
        index: Arc::clone(&index),
    });
    Ok(index)
}

/// Fonte -> PDF.
#[tauri::command]
pub async fn synctex_forward(
    app: AppHandle,
    dir: String,
    file: String,
    line: u32,
) -> Result<Option<ForwardResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<CompileState>();
        let index = load_index(&state, Path::new(&dir))?;
        Ok(index.forward(Path::new(&file), line, Path::new(&dir)))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// PDF -> fonte. `x` e `y` em pontos PDF, a partir do canto superior esquerdo da página.
#[tauri::command]
pub async fn synctex_inverse(
    app: AppHandle,
    dir: String,
    page: u32,
    x: f64,
    y: f64,
) -> Result<Option<SourcePosition>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<CompileState>();
        let index = load_index(&state, Path::new(&dir))?;
        Ok(index.inverse(page, x, y, Path::new(&dir)))
    })
    .await
    .map_err(|e| e.to_string())?
}
