//! Detecção de arquivos faltantes no log do LaTeX/BibTeX e resolução arquivo -> pacote (TL-208).

use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MissingKind {
    Sty,
    Cls,
    Bst,
    Def,
    Font,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingFile {
    pub name: String,
    pub kind: MissingKind,
}

fn kind_of(name: &str) -> MissingKind {
    match name.rsplit('.').next() {
        Some("sty") => MissingKind::Sty,
        Some("cls") => MissingKind::Cls,
        Some("bst") => MissingKind::Bst,
        Some("def") | Some("cfg") | Some("clo") | Some("fd") => MissingKind::Def,
        Some("tfm") | Some("pfb") | Some("otf") | Some("ttf") | Some("pk") => MissingKind::Font,
        _ => MissingKind::Other,
    }
}

/// Extrai os arquivos faltantes de um log, sem repetições e na ordem de aparição.
pub fn find_missing_files(log: &str) -> Vec<MissingFile> {
    let patterns = [
        // ! LaTeX Error: File `siunitx.sty' not found.
        (
            Regex::new(r"! LaTeX Error: File [`'‘]([^'’\s]+)['’] not found").expect("regex"),
            "",
        ),
        // ! I can't find file `foo.def'.
        (
            Regex::new(r"! I can't find file [`'‘]([^'’\s]+)['’]").expect("regex"),
            "",
        ),
        // I couldn't open style file abntex2-alf.bst
        (
            Regex::new(r"I couldn't open style file (\S+)").expect("regex"),
            ".bst",
        ),
        // kpathsea: Running mktextfm ecrm1000
        (
            Regex::new(r"kpathsea: Running mktextfm (\S+)").expect("regex"),
            ".tfm",
        ),
    ];

    let mut hits: Vec<(usize, MissingFile)> = Vec::new();
    for (re, default_ext) in &patterns {
        for caps in re.captures_iter(log) {
            let whole = caps.get(0).expect("grupo 0").start();
            let mut name = caps[1].to_string();
            if !default_ext.is_empty() && !name.ends_with(default_ext) {
                name.push_str(default_ext);
            }
            let kind = kind_of(&name);
            hits.push((whole, MissingFile { name, kind }));
        }
    }
    hits.sort_by_key(|(pos, _)| *pos);

    let mut out: Vec<MissingFile> = Vec::new();
    for (_, file) in hits {
        if !out.iter().any(|f| f.name == file.name) {
            out.push(file);
        }
    }
    out
}

/// Interpreta a saída de `tlmgr search --global --file <arquivo>` e devolve os pacotes
/// que contêm exatamente esse arquivo.
pub fn parse_search_output(output: &str, file: &str) -> Vec<String> {
    let mut packages = Vec::new();
    let mut current: Option<&str> = None;
    for line in output.lines() {
        if line.starts_with(char::is_whitespace) {
            let path = line.trim().replace('\\', "/");
            if let Some(pkg) = current {
                let base = path.rsplit('/').next().unwrap_or("");
                if base == file && !packages.iter().any(|p| p == pkg) {
                    packages.push(pkg.to_string());
                }
            }
        } else if let Some(name) = line.strip_suffix(':') {
            current = if name.contains(' ') { None } else { Some(name) };
        } else {
            current = None;
        }
    }
    packages
}

/// Escolhe o pacote mais específico (ignora coleções e esquemas quando possível).
pub fn choose_package(candidates: &[String]) -> Option<&String> {
    candidates
        .iter()
        .find(|p| !p.starts_with("collection-") && !p.starts_with("scheme-"))
        .or_else(|| candidates.first())
}

/// Cache local arquivo -> pacote (persistível em JSON).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PackageIndex {
    map: HashMap<String, String>,
}

impl PackageIndex {
    pub fn get(&self, file: &str) -> Option<&str> {
        self.map.get(file).map(String::as_str)
    }

    pub fn insert(&mut self, file: &str, package: &str) {
        self.map.insert(file.to_string(), package.to_string());
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("PackageIndex é serializável")
    }

    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_sty_cls_bst_and_fonts_in_order_without_duplicates() {
        let log = "\
! LaTeX Error: File `siunitx.sty' not found.
foo
! LaTeX Error: File `abntex2.cls' not found.
bar
! LaTeX Error: File `siunitx.sty' not found.
kpathsea: Running mktextfm ecrm1000
I couldn't open style file abntex2-alf.bst
";
        let found = find_missing_files(log);
        let names: Vec<_> = found.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "siunitx.sty",
                "abntex2.cls",
                "ecrm1000.tfm",
                "abntex2-alf.bst"
            ]
        );
        assert_eq!(found[0].kind, MissingKind::Sty);
        assert_eq!(found[1].kind, MissingKind::Cls);
        assert_eq!(found[2].kind, MissingKind::Font);
        assert_eq!(found[3].kind, MissingKind::Bst);
    }

    #[test]
    fn accepts_unicode_quotes_and_def_files() {
        let log = "! LaTeX Error: File ‘foo.def’ not found.";
        let found = find_missing_files(log);
        assert_eq!(found[0].name, "foo.def");
        assert_eq!(found[0].kind, MissingKind::Def);
    }

    #[test]
    fn clean_log_has_no_missing_files() {
        assert!(find_missing_files("Output written on main.pdf").is_empty());
    }

    #[test]
    fn resolves_package_from_tlmgr_search() {
        let out = "\
tlmgr: package repository https://mirror.example/tlnet
siunitx:
\ttexmf-dist/tex/latex/siunitx/siunitx.sty
\ttexmf-dist/tex/latex/siunitx/siunitx-core.sty
collection-latexextra:
\ttexmf-dist/tex/latex/siunitx/siunitx.sty
other:
\ttexmf-dist/tex/latex/other/siunitx.sty.bak
";
        let pkgs = parse_search_output(out, "siunitx.sty");
        assert_eq!(pkgs, vec!["siunitx", "collection-latexextra"]);
        assert_eq!(choose_package(&pkgs).map(String::as_str), Some("siunitx"));
        assert!(parse_search_output(out, "nada.sty").is_empty());
    }

    /// Saída real do tlmgr (TeX Live 2026, Windows) para `search --global --file siunitx.sty`.
    #[test]
    fn resolves_package_from_real_tlmgr_output() {
        let real = "tlmgr.pl: package repository https://br.mirrors.cicku.me/ctan/systems/texlive/tlnet (not verified: gpg unavailable)
luatexja:
	texmf-dist/tex/luatex/luatexja/patches/lltjp-siunitx.sty
lwarp:
	texmf-dist/tex/latex/lwarp/lwarp-siunitx.sty
plautopatch:
	texmf-dist/tex/latex/plautopatch/plsiunitx.sty
siunitx:
	texmf-dist/tex/latex/siunitx/siunitx.sty
";
        assert_eq!(parse_search_output(real, "siunitx.sty"), vec!["siunitx"]);
        assert!(parse_search_output(real, "lltjp.sty").is_empty());
    }

    #[test]
    fn index_roundtrips() {
        let mut idx = PackageIndex::default();
        idx.insert("siunitx.sty", "siunitx");
        let back = PackageIndex::from_json(&idx.to_json());
        assert_eq!(back.get("siunitx.sty"), Some("siunitx"));
        assert_eq!(PackageIndex::from_json("lixo").get("x"), None);
    }
}
