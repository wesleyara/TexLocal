//! Uso de disco das árvores do TeX Live e caches (TL-212, espaço livre do wizard).

use std::fs;
use std::path::Path;

/// Soma recursiva dos tamanhos dos arquivos (ignora entradas ilegíveis e não segue symlinks).
pub fn dir_size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    fs::read_dir(path)
        .map(|entries| entries.flatten().map(|e| dir_size(&e.path())).sum())
        .unwrap_or(0)
}

/// Espaço livre, em bytes, no volume que contém `path` (sobe até um ancestral existente).
pub fn free_space(path: &Path) -> Option<u64> {
    let mut p = path;
    while !p.exists() {
        p = p.parent()?;
    }
    fs4::available_space(p).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_nested_files() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("a")).unwrap();
        fs::write(tmp.path().join("a/x"), [0u8; 100]).unwrap();
        fs::write(tmp.path().join("y"), [0u8; 50]).unwrap();
        assert_eq!(dir_size(tmp.path()), 150);
        assert_eq!(dir_size(&tmp.path().join("missing")), 0);
    }

    #[test]
    fn free_space_works_for_nonexistent_child() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(free_space(&tmp.path().join("novo/texlive")).is_some());
    }
}
