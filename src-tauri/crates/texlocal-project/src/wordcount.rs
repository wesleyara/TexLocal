//! Contagem de palavras no estilo `texcount` (TL-310).

use std::path::Path;

use serde::Serialize;

use crate::prose;
use crate::textfile;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    /// Palavras do texto (fora de títulos).
    pub words: usize,
    /// Palavras em títulos de seção.
    pub headers: usize,
    pub inline_math: usize,
    pub display_math: usize,
}

impl Counts {
    fn add(&mut self, other: Counts) {
        self.words += other.words;
        self.headers += other.headers;
        self.inline_math += other.inline_math;
        self.display_math += other.display_math;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionCount {
    pub file: String,
    /// Vazio para o texto anterior à primeira seção do arquivo.
    pub title: String,
    pub level: i8,
    /// Palavras da seção, incluindo o próprio título.
    pub words: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCount {
    pub total: Counts,
    pub sections: Vec<SectionCount>,
}

pub fn count(text: &str) -> Counts {
    let scan = prose::scan(text);
    let headers = scan.tokens.iter().filter(|t| t.header).count();
    Counts {
        words: scan.tokens.len() - headers,
        headers,
        inline_math: scan.inline_math,
        display_math: scan.display_math,
    }
}

/// Palavras por seção de um arquivo.
pub fn by_section(file: &str, text: &str) -> Vec<SectionCount> {
    let scan = prose::scan(text);
    let mut out = Vec::new();
    let bounds: Vec<usize> = scan.headings.iter().map(|h| h.start).collect();
    let intro = scan
        .tokens
        .iter()
        .filter(|t| bounds.first().is_none_or(|&b| t.start < b))
        .count();
    if intro > 0 {
        out.push(SectionCount {
            file: file.to_owned(),
            title: String::new(),
            level: 0,
            words: intro,
        });
    }
    for (i, h) in scan.headings.iter().enumerate() {
        let next = bounds.get(i + 1).copied().unwrap_or(usize::MAX);
        out.push(SectionCount {
            file: file.to_owned(),
            title: h.title.clone(),
            level: h.level,
            words: scan
                .tokens
                .iter()
                .filter(|t| t.start >= h.start && t.start < next)
                .count(),
        });
    }
    out
}

/// Soma os arquivos do documento (na ordem dada), lendo do disco.
pub fn count_files(paths: &[String]) -> ProjectCount {
    let mut result = ProjectCount::default();
    for p in paths {
        let Ok(bytes) = std::fs::read(Path::new(p)) else {
            continue;
        };
        let text = textfile::decode(&bytes).text;
        result.total.add(count(&text));
        result.sections.extend(by_section(p, &text));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conta_texto_e_titulos_separadamente() {
        let c = count("\\section{Uma seção}\nUm texto de cinco palavras.\n");
        assert_eq!((c.words, c.headers), (5, 2));
    }

    #[test]
    fn conta_math_sem_contar_palavras() {
        let c = count("a $x$ b \\[ y \\]");
        assert_eq!(c.words, 2);
        assert_eq!((c.inline_math, c.display_math), (1, 1));
    }

    #[test]
    fn por_secao() {
        let s = by_section(
            "m.tex",
            "intro aqui\n\\section{A}\num dois\n\\subsection{B}\ntres\n",
        );
        let v: Vec<_> = s.iter().map(|x| (x.title.as_str(), x.words)).collect();
        assert_eq!(v, [("", 2), ("A", 3), ("B", 2)]);
    }

    #[test]
    fn soma_arquivos() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a.tex");
        let b = d.path().join("b.tex");
        std::fs::write(&a, "um dois").unwrap();
        std::fs::write(&b, "tres \\cite{x}").unwrap();
        let r = count_files(&[
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ]);
        assert_eq!(r.total.words, 3);
        assert_eq!(r.sections.len(), 2);
    }
}
