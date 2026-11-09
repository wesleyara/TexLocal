//! Diretórios da instalação gerenciada do TeX Live (sem privilégios de administrador).

use std::path::{Path, PathBuf};

use directories::BaseDirs;

/// Raiz dos dados do TexLocal:
/// - Linux: `~/.local/share/texlocal`
/// - Windows: `%LOCALAPPDATA%\TexLocal`
/// - macOS: `~/Library/Application Support/TexLocal`
pub fn data_root() -> Option<PathBuf> {
    let base = BaseDirs::new()?;
    let name = if cfg!(target_os = "linux") {
        "texlocal"
    } else {
        "TexLocal"
    };
    Some(base.data_local_dir().join(name))
}

/// Árvore do TeX Live de um ano: `<raiz>/texlive/<ano>`.
pub fn managed_tree(root: &Path, year: u16) -> PathBuf {
    root.join("texlive").join(year.to_string())
}

/// Subdiretório de binários do TeX Live para a plataforma atual.
pub fn platform_bin_subdir() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "universal-darwin"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64-linux"
    } else {
        "x86_64-linux"
    }
}

/// Diretório `bin` de uma árvore do TeX Live.
pub fn managed_bin_dir(tree: &Path) -> PathBuf {
    tree.join("bin").join(platform_bin_subdir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_and_bin_layout() {
        let tree = managed_tree(Path::new("/data/texlocal"), 2026);
        assert!(tree.ends_with("texlive/2026") || tree.ends_with("texlive\\2026"));
        assert!(managed_bin_dir(&tree).starts_with(&tree));
    }

    #[test]
    fn data_root_is_named_after_app() {
        let root = data_root().expect("diretório de dados do usuário");
        let name = root.file_name().unwrap().to_string_lossy().to_lowercase();
        assert_eq!(name, "texlocal");
    }
}
