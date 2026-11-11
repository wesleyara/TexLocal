//! Comandos Tauri dos snippets (EP05): leitura dos arquivos global/projeto e observação de mudanças.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

/// Evento emitido quando algum arquivo de snippets muda no disco.
pub const CHANGED_EVENT: &str = "snippets-changed";

#[derive(Default)]
pub struct SnippetWatchState {
    generation: AtomicU64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetFile {
    path: String,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetSources {
    global_dir: String,
    /// `None` sem projeto aberto.
    project_dir: Option<String>,
    global: Vec<SnippetFile>,
    project: Vec<SnippetFile>,
}

fn global_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|d| d.join("snippets"))
        .map_err(|e| e.to_string())
}

fn project_dir(root: &str) -> PathBuf {
    Path::new(root).join(".texlocal").join("snippets")
}

fn is_snippet_file(path: &Path) -> bool {
    path.is_file()
        && path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
            e.eq_ignore_ascii_case("json") || e.eq_ignore_ascii_case("code-snippets")
        })
}

/// Arquivos de snippets de uma pasta, em ordem alfabética (ordem de carga determinística).
fn snippet_paths(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_snippet_file(p))
        .collect();
    files.sort_by_key(|p| p.file_name().map(|n| n.to_ascii_lowercase()));
    files
}

fn read_dir_files(dir: &Path) -> Vec<SnippetFile> {
    snippet_paths(dir)
        .into_iter()
        .filter_map(|p| {
            // Texto inválido como UTF-8 vira erro de JSON no painel, não falha de carga.
            let bytes = std::fs::read(&p).ok()?;
            Some(SnippetFile {
                path: p.to_string_lossy().into_owned(),
                text: String::from_utf8_lossy(&bytes)
                    .trim_start_matches('\u{feff}')
                    .to_owned(),
            })
        })
        .collect()
}

#[tauri::command]
pub async fn snippets_load(app: AppHandle, root: Option<String>) -> Result<SnippetSources, String> {
    let global = global_dir(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _ = std::fs::create_dir_all(&global);
        let project = root.as_deref().map(project_dir);
        SnippetSources {
            global: read_dir_files(&global),
            project: project.as_deref().map(read_dir_files).unwrap_or_default(),
            global_dir: global.to_string_lossy().into_owned(),
            project_dir: project.map(|p| p.to_string_lossy().into_owned()),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

type Snapshot = BTreeMap<PathBuf, (SystemTime, u64)>;

fn snapshot(dirs: &[PathBuf]) -> Snapshot {
    let mut map = Snapshot::new();
    for dir in dirs {
        for p in snippet_paths(dir) {
            if let Ok(meta) = std::fs::metadata(&p) {
                map.insert(
                    p,
                    (
                        meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                        meta.len(),
                    ),
                );
            }
        }
    }
    map
}

/// Passa a observar as pastas de snippets (global e do projeto). Cada chamada substitui a anterior.
#[tauri::command]
pub fn snippets_watch(
    app: AppHandle,
    state: State<'_, SnippetWatchState>,
    root: Option<String>,
) -> Result<(), String> {
    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let mut dirs = vec![global_dir(&app)?];
    if let Some(root) = root {
        dirs.push(project_dir(&root));
    }
    std::thread::spawn(move || {
        let mut last = snapshot(&dirs);
        loop {
            std::thread::sleep(Duration::from_millis(1000));
            let current = app
                .state::<SnippetWatchState>()
                .generation
                .load(Ordering::SeqCst);
            if current != generation {
                return;
            }
            let now = snapshot(&dirs);
            if now != last {
                last = now;
                let _ = app.emit(CHANGED_EVENT, ());
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_only_snippet_files_sorted() {
        let dir = std::env::temp_dir().join(format!("texlocal-snip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["b.json", "a.code-snippets", "c.txt"] {
            std::fs::write(dir.join(name), "{}").unwrap();
        }
        let names: Vec<String> = snippet_paths(&dir)
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a.code-snippets", "b.json"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn snapshot_detects_changes() {
        let dir = std::env::temp_dir().join(format!("texlocal-snap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.json"), "{}").unwrap();
        let before = snapshot(std::slice::from_ref(&dir));
        std::fs::write(dir.join("a.json"), "{\"x\":{}}").unwrap();
        assert_ne!(before, snapshot(std::slice::from_ref(&dir)));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
