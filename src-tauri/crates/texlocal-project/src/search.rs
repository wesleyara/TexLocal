//! Busca e substituição no projeto (TL-308), respeitando `.gitignore`.

use std::path::Path;

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

use crate::textfile;
use crate::utf16_len;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub pattern: String,
    #[serde(default)]
    pub regex: bool,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub whole_word: bool,
    /// Globs separados por vírgula (ex.: `*.tex, chapters/**`). Vazio = todos os arquivos de texto.
    #[serde(default)]
    pub glob: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Match {
    pub line: u32,
    pub col: u32,
    pub end_col: u32,
    pub text: String,
    /// Linha com a substituição aplicada (prévia), quando há substituição.
    pub replaced: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMatches {
    pub path: String,
    pub matches: Vec<Match>,
}

pub fn build_regex(q: &SearchQuery) -> Result<Regex, String> {
    if q.pattern.is_empty() {
        return Err("padrão vazio".into());
    }
    let mut pat = if q.regex {
        q.pattern.clone()
    } else {
        regex::escape(&q.pattern)
    };
    if q.whole_word {
        pat = format!(r"\b(?:{pat})\b");
    }
    RegexBuilder::new(&pat)
        .case_insensitive(!q.case_sensitive)
        .multi_line(true)
        .build()
        .map_err(|e| e.to_string())
}

fn walker(root: &Path, glob: &str) -> Result<ignore::Walk, String> {
    let mut b = ignore::WalkBuilder::new(root);
    b.filter_entry(|e| e.file_name() != ".texlocal");
    let globs: Vec<&str> = glob
        .split(',')
        .map(str::trim)
        .filter(|g| !g.is_empty())
        .collect();
    if !globs.is_empty() {
        let mut ov = ignore::overrides::OverrideBuilder::new(root);
        for g in &globs {
            ov.add(g).map_err(|e| e.to_string())?;
        }
        b.overrides(ov.build().map_err(|e| e.to_string())?);
    }
    Ok(b.build())
}

fn read_text(path: &Path) -> Option<textfile::Decoded> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.contains(&0) {
        return None;
    }
    Some(textfile::decode(&bytes))
}

/// Busca em todos os arquivos de texto. `replacement` gera a prévia por linha.
pub fn search(
    root: &Path,
    q: &SearchQuery,
    replacement: Option<&str>,
    max_matches: usize,
) -> Result<Vec<FileMatches>, String> {
    let re = build_regex(q)?;
    let mut out = Vec::new();
    let mut total = 0;
    for entry in walker(root, &q.glob)?.flatten() {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let Some(file) = read_text(entry.path()) else {
            continue;
        };
        let mut matches = Vec::new();
        for (i, line) in file.text.split('\n').enumerate() {
            for m in re.find_iter(line) {
                if m.as_str().is_empty() {
                    continue;
                }
                matches.push(Match {
                    line: i as u32,
                    col: utf16_len(&line[..m.start()]) as u32,
                    end_col: utf16_len(&line[..m.end()]) as u32,
                    text: line.chars().take(300).collect(),
                    replaced: replacement.map(|r| expand(&re, line, r)),
                });
                total += 1;
                if total >= max_matches {
                    break;
                }
            }
            if total >= max_matches {
                break;
            }
        }
        if !matches.is_empty() {
            out.push(FileMatches {
                path: entry.path().to_string_lossy().into_owned(),
                matches,
            });
        }
        if total >= max_matches {
            break;
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn expand(re: &Regex, line: &str, replacement: &str) -> String {
    re.replace_all(line, replacement)
        .chars()
        .take(300)
        .collect()
}

/// Substitui nos arquivos informados. Preserva fim de linha e codificação. Devolve os alterados.
pub fn replace(
    q: &SearchQuery,
    replacement: &str,
    files: &[String],
) -> Result<Vec<String>, String> {
    let re = build_regex(q)?;
    let mut changed = Vec::new();
    for f in files {
        let path = Path::new(f);
        let Some(file) = read_text(path) else {
            continue;
        };
        let new = re.replace_all(&file.text, replacement);
        if new != file.text {
            let bytes = textfile::encode(&new, file.eol, file.encoding, file.bom)
                .map_err(|c| format!("caractere '{c}' não cabe na codificação de {f}"))?;
            std::fs::write(path, bytes).map_err(|e| format!("{f}: {e}"))?;
            changed.push(f.clone());
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(pattern: &str) -> SearchQuery {
        SearchQuery {
            pattern: pattern.into(),
            regex: false,
            case_sensitive: false,
            whole_word: false,
            glob: String::new(),
        }
    }

    fn project() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.tex"), "Gato e gato\nrato\n").unwrap();
        std::fs::write(d.path().join("b.txt"), "gatos\r\nGATO\r\n").unwrap();
        std::fs::write(d.path().join("bin.dat"), b"gato\0\0").unwrap();
        std::fs::create_dir(d.path().join("out")).unwrap();
        std::fs::write(d.path().join("out/c.tex"), "gato").unwrap();
        std::fs::write(d.path().join(".gitignore"), "out/\n").unwrap();
        std::fs::create_dir(d.path().join(".git")).unwrap(); // `ignore` só lê .gitignore em repositórios
        d
    }

    #[test]
    fn busca_literal_ignora_binarios_e_gitignore() {
        let d = project();
        let r = search(d.path(), &q("gato"), None, 100).unwrap();
        let files: Vec<_> = r
            .iter()
            .map(|f| {
                Path::new(&f.path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(files, ["a.tex", "b.txt"]);
        assert_eq!(r[0].matches.len(), 2);
        assert_eq!(r[0].matches[1].col, 7);
    }

    #[test]
    fn palavra_inteira_e_case() {
        let d = project();
        let mut query = q("gato");
        query.whole_word = true;
        query.case_sensitive = true;
        let r = search(d.path(), &query, None, 100).unwrap();
        let total: usize = r.iter().map(|f| f.matches.len()).sum();
        assert_eq!(total, 1);
    }

    #[test]
    fn regex_com_grupos_e_previa() {
        let d = project();
        let mut query = q(r"(g)ato");
        query.regex = true;
        let r = search(d.path(), &query, Some("${1}ata"), 100).unwrap();
        assert_eq!(r[0].matches[0].replaced.as_deref(), Some("Gata e gata"));
    }

    #[test]
    fn filtro_por_glob() {
        let d = project();
        let mut query = q("gato");
        query.glob = "*.txt".into();
        let r = search(d.path(), &query, None, 100).unwrap();
        assert_eq!(r.len(), 1);
        assert!(r[0].path.ends_with("b.txt"));
    }

    #[test]
    fn regex_invalida_vira_erro() {
        let d = project();
        let mut query = q("(");
        query.regex = true;
        assert!(search(d.path(), &query, None, 10).is_err());
    }

    #[test]
    fn substitui_preservando_crlf() {
        let d = project();
        let file = d.path().join("b.txt").to_string_lossy().into_owned();
        let changed = replace(&q("gato"), "cão", std::slice::from_ref(&file)).unwrap();
        assert_eq!(changed, std::slice::from_ref(&file));
        assert_eq!(std::fs::read(&file).unwrap(), "cãos\r\ncão\r\n".as_bytes());
    }

    #[test]
    fn respeita_limite_de_resultados() {
        let d = project();
        let r = search(d.path(), &q("gato"), None, 1).unwrap();
        assert_eq!(r.iter().map(|f| f.matches.len()).sum::<usize>(), 1);
    }
}
