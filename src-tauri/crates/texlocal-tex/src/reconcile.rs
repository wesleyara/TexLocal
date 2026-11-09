//! Reconciliação das dependências declaradas no manifesto com a instalação (TL-207).

use std::collections::HashSet;

use serde::Serialize;

use crate::manifest::Manifest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum YearAction {
    /// Ano instalado atende ao exigido (ou nada foi exigido).
    Satisfied,
    /// O projeto exige um TeX Live mais novo: atualizar ou instalar árvore lado a lado.
    NeedsNewer { required: u16, installed: u16 },
    /// Há ano exigido mas a distribuição não informa ano (ex.: MiKTeX).
    Unknown { required: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcilePlan {
    pub missing_packages: Vec<String>,
    pub year: YearAction,
}

impl ReconcilePlan {
    pub fn is_noop(&self) -> bool {
        self.missing_packages.is_empty() && self.year == YearAction::Satisfied
    }
}

/// Compara o manifesto com os pacotes instalados e o ano do TeX Live.
pub fn plan(
    manifest: &Manifest,
    installed: &HashSet<String>,
    installed_year: Option<u16>,
) -> ReconcilePlan {
    let mut missing: Vec<String> = Vec::new();
    for p in &manifest.tex.packages {
        if !installed.contains(p) && !missing.contains(p) {
            missing.push(p.clone());
        }
    }
    let year = match (manifest.tex.texlive, installed_year) {
        (None, _) => YearAction::Satisfied,
        (Some(req), Some(inst)) if inst >= req => YearAction::Satisfied,
        (Some(required), Some(installed)) => YearAction::NeedsNewer {
            required,
            installed,
        },
        (Some(required), None) => YearAction::Unknown { required },
    };
    ReconcilePlan {
        missing_packages: missing,
        year,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(packages: &[&str], year: Option<u16>) -> Manifest {
        let mut m = Manifest::default();
        m.tex.packages = packages.iter().map(|s| (*s).to_string()).collect();
        m.tex.texlive = year;
        m
    }

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn lists_only_missing_packages_without_duplicates() {
        let m = manifest(&["abntex2", "siunitx", "siunitx"], None);
        let plan = plan(&m, &set(&["abntex2"]), Some(2026));
        assert_eq!(plan.missing_packages, vec!["siunitx"]);
        assert!(!plan.is_noop());
    }

    #[test]
    fn noop_when_everything_is_installed() {
        let m = manifest(&["abntex2"], Some(2025));
        assert!(plan(&m, &set(&["abntex2"]), Some(2026)).is_noop());
    }

    #[test]
    fn newer_year_required() {
        let m = manifest(&[], Some(2027));
        assert_eq!(
            plan(&m, &set(&[]), Some(2026)).year,
            YearAction::NeedsNewer {
                required: 2027,
                installed: 2026
            }
        );
        assert_eq!(
            plan(&m, &set(&[]), None).year,
            YearAction::Unknown { required: 2027 }
        );
    }
}
