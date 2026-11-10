//! Recursos baixados pelo app (TeX Live gerenciado, cache de downloads) e sua remoção segura.
//!
//! Tudo vive sob a raiz de dados (`paths::data_root`). Só é possível remover o que `list`
//! devolve: o identificador de um recurso é validado contra a listagem, nunca usado como caminho.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::disk::dir_size;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceKind {
    /// Árvore do TeX Live instalada pelo app (`texlive/<ano>`).
    TexTree,
    /// Instalação interrompida (`texlive/.<ano>.partial`).
    Partial,
    /// Arquivos baixados e extraídos durante a instalação (`cache`).
    DownloadCache,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    /// Caminho relativo à raiz de dados, com `/`; serve de identificador.
    pub id: String,
    pub kind: ResourceKind,
    /// Ano do TeX Live, quando faz sentido.
    pub year: Option<u16>,
    pub path: String,
    pub size_bytes: u64,
}

fn entry(root: &Path, rel: &str, kind: ResourceKind, year: Option<u16>) -> Option<Resource> {
    let path = root.join(rel);
    if !path.is_dir() {
        return None;
    }
    Some(Resource {
        id: rel.to_owned(),
        kind,
        year,
        path: path.to_string_lossy().into_owned(),
        size_bytes: dir_size(&path),
    })
}

/// Lista os recursos existentes sob `root`, do maior para o menor.
pub fn list(root: &Path) -> Vec<Resource> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(root.join("texlive")) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let rel = format!("texlive/{name}");
            if let Some(rest) = name.strip_prefix('.') {
                let year = rest.strip_suffix(".partial").and_then(|y| y.parse().ok());
                out.extend(entry(root, &rel, ResourceKind::Partial, year));
            } else if let Ok(year) = name.parse::<u16>() {
                out.extend(entry(root, &rel, ResourceKind::TexTree, Some(year)));
            }
        }
    }
    out.extend(entry(root, "cache", ResourceKind::DownloadCache, None));
    out.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes).then(a.id.cmp(&b.id)));
    out
}

/// Remove o recurso `id`. Falha se ele não estiver na listagem.
pub fn remove(root: &Path, id: &str) -> Result<PathBuf, String> {
    let found = list(root)
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| format!("recurso desconhecido: {id}"))?;
    let path = PathBuf::from(&found.path);
    fs::remove_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, bytes: usize) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, vec![0u8; bytes]).unwrap();
    }

    #[test]
    fn lists_trees_partials_and_cache_with_sizes() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "texlive/2026/bin/x", 300);
        write(tmp.path(), "texlive/.2027.partial/y", 10);
        write(tmp.path(), "cache/install-tl.zip", 100);
        write(tmp.path(), "texlive/notes.txt", 5);
        let r = list(tmp.path());
        let ids: Vec<_> = r.iter().map(|x| x.id.as_str()).collect();
        assert_eq!(ids, ["texlive/2026", "cache", "texlive/.2027.partial"]);
        assert_eq!(r[0].kind, ResourceKind::TexTree);
        assert_eq!(r[0].year, Some(2026));
        assert_eq!(r[0].size_bytes, 300);
        assert_eq!(r[2].kind, ResourceKind::Partial);
        assert_eq!(r[2].year, Some(2027));
    }

    #[test]
    fn empty_root_has_no_resources() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(list(tmp.path()).is_empty());
    }

    #[test]
    fn removes_only_listed_resources() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "texlive/2026/a", 1);
        write(tmp.path(), "cache/b", 1);
        write(tmp.path(), "outside/c", 1);
        remove(tmp.path(), "cache").unwrap();
        assert!(!tmp.path().join("cache").exists());
        assert!(remove(tmp.path(), "outside").is_err());
        assert!(remove(tmp.path(), "../outside").is_err());
        assert!(remove(tmp.path(), "texlive/2026/../../outside").is_err());
        assert!(tmp.path().join("outside/c").exists());
        remove(tmp.path(), "texlive/2026").unwrap();
        assert!(!tmp.path().join("texlive/2026").exists());
    }
}
