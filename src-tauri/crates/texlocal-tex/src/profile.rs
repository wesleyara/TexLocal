//! Perfis de instalação do TeX Live e geração do arquivo de profile do `install-tl`.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Minimal,
    Recommended,
    Full,
}

impl Profile {
    pub const ALL: [Profile; 3] = [Profile::Minimal, Profile::Recommended, Profile::Full];

    /// Tamanho aproximado em MB (exibido no assistente).
    pub fn approx_size_mb(self) -> u64 {
        match self {
            Self::Minimal => 150,
            Self::Recommended => 800,
            Self::Full => 5000,
        }
    }

    pub fn scheme(self) -> &'static str {
        match self {
            Self::Minimal => "scheme-basic",
            Self::Recommended => "scheme-medium",
            Self::Full => "scheme-full",
        }
    }

    /// Pacotes garantidos via `tlmgr install` após a instalação base (português e latexmk).
    ///
    /// Não vão no profile do `install-tl`: listar `collection-*` lá substitui a seleção do
    /// esquema (o `scheme-basic` perde o LaTeX), como verificado na instalação real.
    pub fn post_install_packages(self) -> &'static [&'static str] {
        match self {
            // O esquema completo já inclui tudo isto.
            Self::Full => &[],
            _ => &["collection-langportuguese", "latexmk"],
        }
    }

    /// Gera o conteúdo do arquivo de profile do `install-tl` para instalação portátil em modo usuário.
    pub fn install_profile(self, tree: &Path) -> String {
        let t = tree.to_string_lossy().replace('\\', "/");
        let mut lines = vec![
            format!("selected_scheme {}", self.scheme()),
            format!("TEXDIR {t}"),
            format!("TEXMFLOCAL {t}/texmf-local"),
            format!("TEXMFSYSCONFIG {t}/texmf-config"),
            format!("TEXMFSYSVAR {t}/texmf-var"),
            format!("TEXMFCONFIG {t}/texmf-config"),
            format!("TEXMFVAR {t}/texmf-var"),
            "instopt_adjustpath 0".into(),
            "instopt_adjustrepo 1".into(),
            "instopt_letter 0".into(),
            "instopt_portable 1".into(),
            "instopt_write18_restricted 1".into(),
            "tlpdbopt_autobackup 0".into(),
            "tlpdbopt_desktop_integration 0".into(),
            "tlpdbopt_file_assocs 0".into(),
            "tlpdbopt_install_docfiles 0".into(),
            "tlpdbopt_install_srcfiles 0".into(),
        ];
        lines.push(String::new());
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_schemes_match_the_wizard_profiles() {
        assert_eq!(Profile::Minimal.approx_size_mb(), 150);
        assert_eq!(Profile::Recommended.approx_size_mb(), 800);
        assert_eq!(Profile::Full.approx_size_mb(), 5000);
        assert_eq!(Profile::Full.scheme(), "scheme-full");
    }

    #[test]
    fn profile_is_portable_user_mode_and_only_selects_the_scheme() {
        for p in Profile::ALL {
            let text = p.install_profile(Path::new("/data/texlive/2026"));
            assert!(text.contains("instopt_portable 1"));
            assert!(text.contains("instopt_adjustpath 0"));
            // collection-* no profile substituiria a seleção do esquema.
            assert!(!text.contains("collection-"));
            assert!(text.contains("TEXDIR /data/texlive/2026"));
            assert!(text.ends_with('\n') && !text.contains('\r'));
        }
        assert!(Profile::Minimal
            .post_install_packages()
            .contains(&"latexmk"));
        assert!(Profile::Recommended
            .post_install_packages()
            .contains(&"collection-langportuguese"));
        assert!(Profile::Full.post_install_packages().is_empty());
    }
}
