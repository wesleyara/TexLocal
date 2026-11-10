//! Comandos Tauri do editor (EP03): arquivos, índice do projeto, busca, palavras e ortografia.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use texlocal_project::index::{Indexer, ProjectView};
use texlocal_project::search::{self, FileMatches, SearchQuery};
use texlocal_project::spell::{self, Misspelling, Speller};
use texlocal_project::textfile::{self, Encoding, Eol};
use texlocal_project::wordcount::{self, Counts, ProjectCount};
use texlocal_tex::{manifest, paths};

#[derive(Default)]
pub struct ProjectState {
    indexer: Mutex<Indexer>,
    manifest_main: Mutex<Option<String>>,
    speller: Mutex<Speller>,
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

fn lock<T>(m: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, String> {
    m.lock().map_err(|_| "estado interno corrompido".to_owned())
}

// ───────────────────────── arquivos ─────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    text: String,
    eol: Eol,
    encoding: Encoding,
    bom: bool,
}

#[tauri::command]
pub async fn file_read(path: String, encoding: Option<Encoding>) -> Result<FileContent, String> {
    blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
        let d = textfile::decode_as(&bytes, encoding);
        Ok(FileContent {
            text: d.text,
            eol: d.eol,
            encoding: d.encoding,
            bom: d.bom,
        })
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteResult {
    /// Codificação realmente usada (cai para UTF-8 se o texto não cabe na escolhida).
    encoding: Encoding,
}

#[tauri::command]
pub async fn file_write(
    path: String,
    text: String,
    eol: Eol,
    encoding: Encoding,
    bom: bool,
) -> Result<WriteResult, String> {
    blocking(move || {
        let (bytes, used) = match textfile::encode(&text, eol, encoding, bom) {
            Ok(b) => (b, encoding),
            Err(_) => (
                textfile::encode(&text, eol, Encoding::Utf8, false).map_err(|c| c.to_string())?,
                Encoding::Utf8,
            ),
        };
        let target = PathBuf::from(&path);
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        // Escrita atômica: arquivo temporário vizinho + rename.
        let tmp = target.with_extension("texlocal-tmp");
        std::fs::write(&tmp, bytes).map_err(|e| format!("{path}: {e}"))?;
        std::fs::rename(&tmp, &target).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("{path}: {e}")
        })?;
        Ok(WriteResult { encoding: used })
    })
    .await
}

/// Bytes do arquivo, sem decodificar (imagens e PDFs da visualização).
#[tauri::command]
pub async fn file_read_bytes(path: String) -> Result<tauri::ipc::Response, String> {
    blocking(move || {
        std::fs::read(&path)
            .map(tauri::ipc::Response::new)
            .map_err(|e| format!("{path}: {e}"))
    })
    .await
}

#[tauri::command]
pub fn path_exists(path: String) -> bool {
    Path::new(&path).is_file()
}

/// Mostra o arquivo no gerenciador de arquivos do sistema (Explorer/Finder), já selecionado.
#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    if !target.exists() {
        return Err(format!("{path}: arquivo não encontrado"));
    }
    #[cfg(windows)]
    {
        // O `explorer /select,` só entende barras invertidas.
        let native = path.replace('/', "\\");
        std::process::Command::new("explorer")
            .arg(format!("/select,{native}"))
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&target)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let dir = target.parent().unwrap_or(&target);
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Lista os arquivos do projeto (relativos à raiz), respeitando `.gitignore`.
#[tauri::command]
pub async fn project_files(root: String) -> Result<Vec<String>, String> {
    blocking(move || {
        let root = PathBuf::from(root);
        let relative = |e: ignore::DirEntry| -> Option<String> {
            if !e.file_type().is_some_and(|t| t.is_file()) {
                return None;
            }
            let rel = e.path().strip_prefix(&root).ok()?;
            Some(rel.to_string_lossy().replace('\\', "/"))
        };
        // Arquivos do projeto: mostra os ocultos (`.vscode`…), respeita o `.gitignore`.
        let mut files: Vec<String> = ignore::WalkBuilder::new(&root)
            .hidden(false)
            .filter_entry(|e| e.file_name() != ".texlocal" && e.file_name() != ".git")
            .build()
            .flatten()
            .filter_map(relative)
            .collect();
        // `.texlocal` (manifesto, build e PDF) está no `.gitignore` do projeto, mas deve aparecer.
        let texlocal = root.join(".texlocal");
        if texlocal.is_dir() {
            files.extend(
                ignore::WalkBuilder::new(&texlocal)
                    .standard_filters(false)
                    .build()
                    .flatten()
                    .filter_map(relative),
            );
        }
        files.sort_by_key(|f| f.to_lowercase());
        Ok(files)
    })
    .await
}

// ───────────────────────── índice ─────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedProject {
    root: String,
    view: ProjectView,
    /// Idioma do manifesto (`[editor] language`), se definido.
    language: Option<String>,
}

fn current_view(
    state: &ProjectState,
    root: &Path,
    hint: Option<&Path>,
) -> Result<ProjectView, String> {
    let manifest_main = lock(&state.manifest_main)?.clone();
    let ix = lock(&state.indexer)?;
    let main = ix.detect_main(root, manifest_main.as_deref(), hint);
    Ok(ix.view(root, main.as_deref()))
}

#[tauri::command]
pub async fn project_open(
    state: State<'_, ProjectState>,
    root: String,
) -> Result<OpenedProject, String> {
    let root_path = PathBuf::from(&root);
    let (manifest_main, language) = match manifest::read(&root_path) {
        Ok(Some(m)) => (
            Some(m.project.main.clone()),
            m.editor
                .get("language")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
        ),
        _ => (None, None),
    };
    *lock(&state.manifest_main)? = manifest_main;
    let scan_root = root_path.clone();
    // O Indexer fica no estado gerenciado; o escaneamento roda numa thread bloqueante.
    let scanned = blocking(move || {
        let mut ix = Indexer::default();
        ix.scan(&scan_root);
        Ok(ix)
    })
    .await?;
    *lock(&state.indexer)? = scanned;
    let view = current_view(&state, &root_path, None)?;
    Ok(OpenedProject {
        root,
        view,
        language,
    })
}

/// Atualização incremental: reanalisa só `path` com o texto do buffer.
#[tauri::command]
pub fn index_update(
    state: State<'_, ProjectState>,
    root: String,
    path: String,
    text: String,
    set_hint: bool,
) -> Result<ProjectView, String> {
    lock(&state.indexer)?.update(Path::new(&path), &text);
    let hint = set_hint.then(|| PathBuf::from(&path));
    current_view(&state, Path::new(&root), hint.as_deref())
}

#[tauri::command]
pub fn index_remove(
    state: State<'_, ProjectState>,
    root: String,
    path: String,
) -> Result<ProjectView, String> {
    lock(&state.indexer)?.remove(Path::new(&path));
    current_view(&state, Path::new(&root), None)
}

// ───────────────────────── busca ─────────────────────────

#[tauri::command]
pub async fn project_search(
    root: String,
    query: SearchQuery,
    replacement: Option<String>,
) -> Result<Vec<FileMatches>, String> {
    blocking(move || search::search(Path::new(&root), &query, replacement.as_deref(), 2000)).await
}

#[tauri::command]
pub async fn project_replace(
    query: SearchQuery,
    replacement: String,
    files: Vec<String>,
) -> Result<Vec<String>, String> {
    blocking(move || search::replace(&query, &replacement, &files)).await
}

// ───────────────────────── palavras ─────────────────────────

#[tauri::command]
pub fn word_count_text(text: String) -> Counts {
    wordcount::count(&text)
}

#[tauri::command]
pub async fn word_count_files(paths: Vec<String>) -> Result<ProjectCount, String> {
    blocking(move || Ok(wordcount::count_files(&paths))).await
}

// ───────────────────────── ortografia ─────────────────────────

fn global_dictionary(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|d| d.join("dictionary.txt"))
        .map_err(|e| e.to_string())
}

fn project_dictionary(root: &str) -> PathBuf {
    Path::new(root).join(".texlocal").join("dictionary.txt")
}

/// Configura pastas de dicionários e palavras pessoais (global + projeto).
#[tauri::command]
pub fn spell_configure(
    app: AppHandle,
    state: State<'_, ProjectState>,
    root: Option<String>,
) -> Result<Vec<String>, String> {
    let mut dirs = Vec::new();
    if let Ok(res) = app.path().resource_dir() {
        dirs.push(res.join("dictionaries"));
    }
    if let Some(data) = paths::data_root() {
        dirs.push(data.join("dictionaries"));
    }
    let mut personal = spell::read_personal(&global_dictionary(&app)?);
    if let Some(r) = &root {
        personal.extend(spell::read_personal(&project_dictionary(r)));
    }
    let mut speller = lock(&state.speller)?;
    *speller = Speller::new(dirs);
    speller.set_personal(personal);
    Ok(speller.available())
}

#[tauri::command]
pub fn spell_check(
    state: State<'_, ProjectState>,
    text: String,
    lang: String,
) -> Result<Vec<Misspelling>, String> {
    lock(&state.speller)?.check_text(&text, &lang)
}

#[tauri::command]
pub fn spell_suggest(
    state: State<'_, ProjectState>,
    word: String,
    lang: String,
) -> Result<Vec<String>, String> {
    lock(&state.speller)?.suggest(&word, &lang)
}

#[tauri::command]
pub fn spell_detect_language(text: String) -> Option<String> {
    spell::detect_language(&text)
}

/// Adiciona ao dicionário pessoal: `scope` = "global" ou "project".
#[tauri::command]
pub fn spell_add_word(
    app: AppHandle,
    state: State<'_, ProjectState>,
    word: String,
    scope: String,
    root: Option<String>,
) -> Result<(), String> {
    let path = match (scope.as_str(), root) {
        ("project", Some(r)) => project_dictionary(&r),
        ("global", _) => global_dictionary(&app)?,
        _ => return Err("escopo inválido ou projeto não aberto".into()),
    };
    spell::append_personal(&path, &word).map_err(|e| e.to_string())?;
    lock(&state.speller)?.add_personal(&word);
    Ok(())
}
