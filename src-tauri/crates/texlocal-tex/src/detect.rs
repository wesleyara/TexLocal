//! Detecção de distribuições TeX já instaladas (TeX Live, MiKTeX, MacTeX).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DistKind {
    TexLive,
    MikTex,
    MacTex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Distribution {
    pub kind: DistKind,
    /// Ano do TeX Live (ausente no MiKTeX).
    pub year: Option<u16>,
    pub bin_dir: PathBuf,
    pub engines: Vec<String>,
    pub has_tlmgr: bool,
    pub has_latexmk: bool,
}

const ENGINES: [&str; 3] = ["pdflatex", "xelatex", "lualatex"];

/// Interpreta a saída de `pdflatex --version`.
pub fn parse_version_output(output: &str, bin_dir: &Path) -> Option<(DistKind, Option<u16>)> {
    if output.contains("MiKTeX") {
        return Some((DistKind::MikTex, None));
    }
    let idx = output.find("TeX Live ")?;
    let year: u16 = output[idx + "TeX Live ".len()..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()?;
    let path = bin_dir.to_string_lossy().replace('\\', "/");
    let kind = if path.contains("/Library/TeX/") {
        DistKind::MacTex
    } else {
        DistKind::TexLive
    };
    Some((kind, Some(year)))
}

/// Procura um executável (`name`, `name.exe`, `name.bat`, `name.cmd`) em `dir`.
pub fn find_executable(dir: &Path, name: &str) -> Option<PathBuf> {
    ["", ".exe", ".bat", ".cmd"]
        .iter()
        .map(|ext| dir.join(format!("{name}{ext}")))
        .find(|p| p.is_file())
}

/// Diretórios candidatos: PATH + locais padrão + `extra` (ex.: árvores gerenciadas).
pub fn candidate_bin_dirs(path_env: &str, extra: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::split_paths(path_env).collect();
    dirs.extend(extra.iter().cloned());
    dirs.extend(standard_locations());
    dirs
}

fn standard_locations() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let sub = crate::paths::platform_bin_subdir();
    let mut globs: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        globs.push(PathBuf::from(r"C:\texlive"));
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            out.push(PathBuf::from(&local).join(r"Programs\MiKTeX\miktex\bin\x64"));
        }
        out.push(PathBuf::from(r"C:\Program Files\MiKTeX\miktex\bin\x64"));
    } else {
        globs.push(PathBuf::from("/usr/local/texlive"));
        out.push(PathBuf::from("/Library/TeX/texbin"));
        out.push(PathBuf::from("/usr/bin"));
    }
    for root in globs {
        if let Ok(entries) = std::fs::read_dir(&root) {
            for year in entries.flatten() {
                out.push(year.path().join("bin").join(sub));
            }
        }
    }
    out
}

/// Inspeciona um diretório `bin`; `None` se não houver uma distribuição TeX válida.
pub fn probe(bin_dir: &Path) -> Option<Distribution> {
    let pdflatex = find_executable(bin_dir, "pdflatex")?;
    let output = command(&pdflatex).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let (kind, year) = parse_version_output(&text, bin_dir)?;
    let engines = ENGINES
        .iter()
        .filter(|e| find_executable(bin_dir, e).is_some())
        .map(|e| (*e).to_string())
        .collect();
    Some(Distribution {
        kind,
        year,
        bin_dir: bin_dir.to_path_buf(),
        engines,
        has_tlmgr: find_executable(bin_dir, "tlmgr").is_some()
            || find_executable(bin_dir, "mpm").is_some(),
        has_latexmk: find_executable(bin_dir, "latexmk").is_some(),
    })
}

/// Detecta distribuições nos diretórios candidatos, sem duplicatas.
pub fn detect(candidates: &[PathBuf]) -> Vec<Distribution> {
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    for dir in candidates {
        let key = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.clone());
        if !dir.is_dir() || !seen.insert(key) {
            continue;
        }
        if let Some(d) = probe(dir) {
            found.push(d);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_texlive_year() {
        let out = "pdfTeX 3.141592653-2.6-1.40.26 (TeX Live 2024)\nkpathsea version 6.4.0";
        let (kind, year) = parse_version_output(out, Path::new("/usr/local/texlive/2024/bin"))
            .expect("versão reconhecida");
        assert_eq!(kind, DistKind::TexLive);
        assert_eq!(year, Some(2024));
    }

    #[test]
    fn recognizes_mactex_by_path() {
        let out = "pdfTeX 3.14 (TeX Live 2025)";
        let (kind, _) = parse_version_output(out, Path::new("/Library/TeX/texbin")).unwrap();
        assert_eq!(kind, DistKind::MacTex);
    }

    #[test]
    fn recognizes_miktex() {
        let out = "MiKTeX-pdfTeX 4.20 (MiKTeX 24.1)";
        let (kind, year) = parse_version_output(out, Path::new("C:/miktex/bin")).unwrap();
        assert_eq!(kind, DistKind::MikTex);
        assert_eq!(year, None);
    }

    #[test]
    fn rejects_unknown_output() {
        assert!(parse_version_output("hello", Path::new("/bin")).is_none());
    }

    #[test]
    fn detect_ignores_missing_and_empty_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let found = detect(&[tmp.path().to_path_buf(), tmp.path().join("nope")]);
        assert!(found.is_empty());
    }
}
