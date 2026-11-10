//! Corretor ortográfico (TL-309) com dicionários Hunspell via `spellbook`.
//!
//! Os dicionários (`<idioma>.aff` + `<idioma>.dic`, ex. `pt_BR`, `en_US`) são procurados nas
//! pastas informadas — o app aponta para os recursos embutidos e para o diretório de dados.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use serde::Serialize;
use spellbook::Dictionary;

use crate::prose;
use crate::utf16_len;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Misspelling {
    /// Offsets em unidades UTF-16 no texto inteiro.
    pub from: usize,
    pub to: usize,
    pub word: String,
}

#[derive(Default)]
pub struct Speller {
    dirs: Vec<PathBuf>,
    dicts: HashMap<String, Dictionary>,
    /// Palavras pessoais, em minúsculas.
    personal: HashSet<String>,
}

fn file_stem(lang: &str) -> String {
    lang.replace('-', "_")
}

impl Speller {
    pub fn new(dirs: Vec<PathBuf>) -> Self {
        Self {
            dirs,
            ..Self::default()
        }
    }

    /// Idiomas com dicionário completo (`.aff` e `.dic`) disponível, no formato `pt-BR`.
    pub fn available(&self) -> Vec<String> {
        let mut langs = HashSet::new();
        for dir in &self.dirs {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "aff") && p.with_extension("dic").is_file() {
                    if let Some(stem) = p.file_stem() {
                        langs.insert(stem.to_string_lossy().replace('_', "-"));
                    }
                }
            }
        }
        let mut v: Vec<_> = langs.into_iter().collect();
        v.sort();
        v
    }

    fn ensure(&mut self, lang: &str) -> Result<&Dictionary, String> {
        if !self.dicts.contains_key(lang) {
            let stem = file_stem(lang);
            let dir = self
                .dirs
                .iter()
                .find(|d| d.join(format!("{stem}.aff")).is_file())
                .ok_or_else(|| format!("dicionário '{lang}' não encontrado"))?;
            let aff = std::fs::read_to_string(dir.join(format!("{stem}.aff")))
                .map_err(|e| e.to_string())?;
            let dic = std::fs::read_to_string(dir.join(format!("{stem}.dic")))
                .map_err(|e| e.to_string())?;
            let dict = Dictionary::new(&aff, &dic).map_err(|e| e.to_string())?;
            self.dicts.insert(lang.to_owned(), dict);
        }
        Ok(&self.dicts[lang])
    }

    pub fn set_personal(&mut self, words: impl IntoIterator<Item = String>) {
        self.personal = words.into_iter().map(|w| w.to_lowercase()).collect();
    }

    pub fn add_personal(&mut self, word: &str) {
        self.personal.insert(word.to_lowercase());
    }

    /// Verifica o texto LaTeX, ignorando tudo que não é prosa.
    pub fn check_text(&mut self, text: &str, lang: &str) -> Result<Vec<Misspelling>, String> {
        let personal = self.personal.clone();
        let dict = self.ensure(lang)?;
        let scan = prose::scan(text);
        let mut out = Vec::new();
        let (mut byte, mut units) = (0usize, 0usize);
        for t in scan.tokens.iter().filter(|t| !t.touches_escape) {
            if t.text.chars().any(|c| c.is_ascii_digit()) {
                continue;
            }
            let normalized = t.text.replace('’', "'");
            let bad = normalized
                .split('-')
                .filter(|p| p.chars().count() > 1)
                .any(|p| !is_correct(dict, &personal, p));
            if !bad {
                continue;
            }
            units += utf16_len(&text[byte..t.start]);
            let from = units;
            units += utf16_len(&text[t.start..t.end]);
            byte = t.end;
            out.push(Misspelling {
                from,
                to: units,
                word: t.text.clone(),
            });
        }
        Ok(out)
    }

    pub fn suggest(&mut self, word: &str, lang: &str) -> Result<Vec<String>, String> {
        let dict = self.ensure(lang)?;
        let mut out = Vec::new();
        dict.suggest(word, &mut out);
        out.truncate(8);
        Ok(out)
    }
}

fn is_correct(dict: &Dictionary, personal: &HashSet<String>, word: &str) -> bool {
    // Siglas (todas maiúsculas) e palavras pessoais são aceitas.
    if word.chars().all(|c| !c.is_lowercase()) || personal.contains(&word.to_lowercase()) {
        return true;
    }
    dict.check(word)
}

/// Detecta o idioma pelo `babel`/`polyglossia`/`\selectlanguage` (a última língua listada é a principal).
pub fn detect_language(text: &str) -> Option<String> {
    static BABEL: OnceLock<Regex> = OnceLock::new();
    static POLY: OnceLock<Regex> = OnceLock::new();
    let babel = BABEL.get_or_init(|| {
        Regex::new(r"(?m)^[^%\n]*\\usepackage\s*\[([^\]]*)\]\s*\{[^}]*babel[^}]*\}").unwrap()
    });
    let poly = POLY.get_or_init(|| {
        Regex::new(r"(?m)^[^%\n]*\\setmainlanguage(?:\[[^\]]*\])?\s*\{([^}]*)\}").unwrap()
    });
    let opts = babel
        .captures(text)
        .or_else(|| poly.captures(text))
        .map(|c| c[1].to_owned())?;
    opts.split(',').rev().find_map(|o| {
        let o = o.trim().to_lowercase();
        if o.contains("brazil") || o.contains("portug") {
            Some("pt-BR".to_owned())
        } else if o.contains("english") || o.contains("american") || o.contains("british") {
            Some("en-US".to_owned())
        } else {
            None
        }
    })
}

/// Lê o dicionário pessoal (uma palavra por linha).
pub fn read_personal(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|s| {
            s.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Acrescenta uma palavra ao dicionário pessoal, sem duplicar.
pub fn append_personal(path: &Path, word: &str) -> std::io::Result<()> {
    let mut words = read_personal(path);
    if words.iter().any(|w| w.eq_ignore_ascii_case(word)) {
        return Ok(());
    }
    words.push(word.to_owned());
    words.sort_by_key(|w| w.to_lowercase());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, words.join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn speller() -> (tempfile::TempDir, Speller) {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("pt_BR.aff"), "SET UTF-8\n").unwrap();
        std::fs::write(
            d.path().join("pt_BR.dic"),
            "6
gato
casa
ação
bem
vindo
em
",
        )
        .unwrap();
        let s = Speller::new(vec![d.path().to_path_buf()]);
        (d, s)
    }

    #[test]
    fn lista_idiomas_disponiveis() {
        let (_d, s) = speller();
        assert_eq!(s.available(), ["pt-BR"]);
    }

    #[test]
    fn aponta_apenas_erros_em_prosa() {
        let (_d, mut s) = speller();
        let src = "O gato \\textbf{gatoo} em $xyz$ \\ref{naoexiste} ação";
        let r = s.check_text(src, "pt-BR").unwrap();
        let words: Vec<_> = r.iter().map(|m| m.word.as_str()).collect();
        assert_eq!(words, ["gatoo"]);
        let m = &r[0];
        assert_eq!(&src[m.from..m.to], "gatoo");
    }

    #[test]
    fn offsets_em_utf16() {
        let (_d, mut s) = speller();
        let r = s.check_text("😀 gatoo", "pt-BR").unwrap();
        assert_eq!((r[0].from, r[0].to), (3, 8));
    }

    #[test]
    fn dicionario_pessoal_e_siglas() {
        let (_d, mut s) = speller();
        assert_eq!(s.check_text("gatoo TCC", "pt-BR").unwrap().len(), 1);
        s.add_personal("Gatoo");
        assert!(s.check_text("gatoo TCC", "pt-BR").unwrap().is_empty());
    }

    #[test]
    fn palavras_com_hifen_checam_cada_parte() {
        let (_d, mut s) = speller();
        assert!(s.check_text("bem-vindo", "pt-BR").unwrap().is_empty());
        assert_eq!(s.check_text("bem-vindoo", "pt-BR").unwrap().len(), 1);
    }

    #[test]
    fn idioma_ausente_vira_erro() {
        let (_d, mut s) = speller();
        assert!(s.check_text("x", "en-US").is_err());
    }

    #[test]
    fn detecta_idioma_pelo_babel() {
        assert_eq!(
            detect_language("\\usepackage[brazil]{babel}").as_deref(),
            Some("pt-BR")
        );
        assert_eq!(
            detect_language("\\usepackage[portuguese,english]{babel}").as_deref(),
            Some("en-US")
        );
        assert_eq!(detect_language("% \\usepackage[brazil]{babel}"), None);
        assert_eq!(detect_language("\\usepackage{amsmath}"), None);
    }

    #[test]
    fn dicionario_pessoal_em_arquivo() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join(".texlocal/dictionary.txt");
        append_personal(&p, "Zebra").unwrap();
        append_personal(&p, "abacate").unwrap();
        append_personal(&p, "zebra").unwrap();
        assert_eq!(read_personal(&p), ["abacate", "Zebra"]);
    }
}
