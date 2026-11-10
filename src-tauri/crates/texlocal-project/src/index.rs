//! Indexador tolerante de projetos LaTeX (TL-307): parser por arquivo + resolução da árvore
//! a partir do arquivo principal. Alimenta outline, referências, comandos e citações.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use serde::Serialize;

use crate::textfile;
use crate::utf16_len;

/// Posição de um trecho em uma linha (colunas em unidades UTF-16, como o CodeMirror).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Span {
    pub line: u32,
    pub col: u32,
    pub end_col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    /// part, chapter, section, subsection, ...
    pub kind: String,
    /// -1 (part) .. 5 (subparagraph)
    pub level: i8,
    pub title: String,
    pub starred: bool,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub name: String,
    pub span: Span,
    /// Prévia: legenda do float, título da seção ou texto da linha.
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub name: String,
    pub command: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    pub key: String,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IncludeKind {
    Input,
    Include,
    Subfile,
    Import,
    Graphics,
    Bibliography,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Include {
    pub kind: IncludeKind,
    /// Caminho como escrito (para `\import`, já combinado: `dir/arquivo`).
    pub path: String,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub name: String,
    pub options: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UserCommandKind {
    Command,
    Operator,
    Environment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCommand {
    pub name: String,
    pub kind: UserCommandKind,
    pub args: u8,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Float {
    /// figure ou table
    pub kind: String,
    pub caption: String,
    pub label: Option<String>,
    pub line: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileIndex {
    pub document_class: Option<String>,
    /// Valor de `% !TEX root = ...`.
    pub magic_root: Option<String>,
    pub sections: Vec<Section>,
    pub labels: Vec<Label>,
    pub references: Vec<Reference>,
    pub citations: Vec<Citation>,
    pub includes: Vec<Include>,
    pub packages: Vec<Package>,
    pub commands: Vec<UserCommand>,
    pub floats: Vec<Float>,
}

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("regex válida"))
}

fn blank(s: &str) -> String {
    " ".repeat(s.len())
}

/// Substitui por espaços (preservando bytes e quebras de linha) comentários e verbatim.
fn mask(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let mut backslashes = 0;
        let mut cut = None;
        for (j, c) in line.char_indices() {
            if c == '%' && backslashes % 2 == 0 {
                cut = Some(j);
                break;
            }
            backslashes = if c == '\\' { backslashes + 1 } else { 0 };
        }
        match cut {
            Some(j) => {
                out.push_str(&line[..j]);
                out.push_str(&blank(&line[j..]));
            }
            None => out.push_str(line),
        }
    }
    mask_verbatim(&out)
}

fn mask_verbatim(text: &str) -> String {
    static BEGIN: OnceLock<Regex> = OnceLock::new();
    let begin = re(
        &BEGIN,
        r"\\begin\{(verbatim\*?|Verbatim\*?|lstlisting|minted|comment)\}",
    );
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    while let Some(m) = begin.captures_at(text, pos) {
        let whole = m.get(0).unwrap();
        let end_tag = format!("\\end{{{}}}", &m[1]);
        let body_start = whole.end();
        let body_end = text[body_start..]
            .find(&end_tag)
            .map_or(text.len(), |p| body_start + p);
        out.push_str(&text[pos..body_start]);
        for line in text[body_start..body_end].split_inclusive('\n') {
            match line.strip_suffix('\n') {
                Some(l) => {
                    out.push_str(&blank(l));
                    out.push('\n');
                }
                None => out.push_str(&blank(line)),
            }
        }
        pos = body_end;
        if pos >= text.len() {
            break;
        }
    }
    out.push_str(&text[pos..]);
    out
}

struct Locator<'a> {
    orig: &'a str,
    starts: Vec<usize>,
}

impl<'a> Locator<'a> {
    fn new(orig: &'a str) -> Self {
        let mut starts = vec![0];
        starts.extend(orig.match_indices('\n').map(|(i, _)| i + 1));
        Self { orig, starts }
    }
    fn line(&self, byte: usize) -> u32 {
        (self.starts.partition_point(|&s| s <= byte) - 1) as u32
    }
    fn col(&self, byte: usize) -> u32 {
        let start = self.starts[self.line(byte) as usize];
        utf16_len(self.orig.get(start..byte).unwrap_or("")) as u32
    }
    fn span(&self, from: usize, to: usize) -> Span {
        Span {
            line: self.line(from),
            col: self.col(from),
            end_col: self.col(to),
        }
    }
}

/// Lê um grupo `{...}` balanceado cujo `{` está em `open`. Devolve (conteúdo, posição após `}`).
pub(crate) fn read_group(text: &str, open: usize) -> Option<(std::ops::Range<usize>, usize)> {
    let bytes = text.as_bytes();
    if bytes.get(open) != Some(&b'{') {
        return None;
    }
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((open + 1..i, i + 1));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

const SECTION_KINDS: [(&str, i8); 7] = [
    ("part", -1),
    ("chapter", 0),
    ("section", 1),
    ("subsection", 2),
    ("subsubsection", 3),
    ("paragraph", 4),
    ("subparagraph", 5),
];

/// Itens separados por vírgula dentro de um grupo, com seus offsets em bytes.
fn comma_items(text: &str, start: usize) -> Vec<(String, usize, usize)> {
    let mut out = Vec::new();
    let mut offset = start;
    for part in text.split(',') {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            let from = offset + (part.len() - part.trim_start().len());
            out.push((trimmed.to_owned(), from, from + trimmed.len()));
        }
        offset += part.len() + 1;
    }
    out
}

/// Analisa um arquivo `.tex` (ou `.sty`/`.cls`), tolerando erros de sintaxe.
pub fn parse(source: &str) -> FileIndex {
    static MAGIC: OnceLock<Regex> = OnceLock::new();
    static CLASS: OnceLock<Regex> = OnceLock::new();
    static SECTION: OnceLock<Regex> = OnceLock::new();
    static LABEL: OnceLock<Regex> = OnceLock::new();
    static REF: OnceLock<Regex> = OnceLock::new();
    static CITE: OnceLock<Regex> = OnceLock::new();
    static INCLUDE: OnceLock<Regex> = OnceLock::new();
    static IMPORT: OnceLock<Regex> = OnceLock::new();
    static PACKAGE: OnceLock<Regex> = OnceLock::new();
    static NEWCMD: OnceLock<Regex> = OnceLock::new();
    static MATHOP: OnceLock<Regex> = OnceLock::new();
    static NEWENV: OnceLock<Regex> = OnceLock::new();
    static FLOAT: OnceLock<Regex> = OnceLock::new();
    static CAPTION: OnceLock<Regex> = OnceLock::new();
    static FLOAT_LABEL: OnceLock<Regex> = OnceLock::new();

    let masked = mask(source);
    let loc = Locator::new(source);
    let mut idx = FileIndex::default();

    // Comentário mágico (no texto original: ele é um comentário).
    for line in source.lines().take(20) {
        if let Some(c) = re(&MAGIC, r"(?i)^\s*%\s*!\s*TEX\s+root\s*=\s*(.+?)\s*$").captures(line) {
            idx.magic_root = Some(c[1].to_owned());
            break;
        }
    }

    if let Some(c) =
        re(&CLASS, r"\\documentclass\s*(?:\[[^\]]*\])?\s*\{([^}]*)\}").captures(&masked)
    {
        idx.document_class = Some(c[1].trim().to_owned());
    }

    let section = re(
        &SECTION,
        r"\\(part|chapter|section|subsection|subsubsection|paragraph|subparagraph)(\*?)\s*(?:\[[^\]]*\])?\s*\{",
    );
    for c in section.captures_iter(&masked) {
        let whole = c.get(0).unwrap();
        let Some((range, _)) = read_group(&masked, whole.end() - 1) else {
            continue;
        };
        let kind = &c[1];
        let level = SECTION_KINDS
            .iter()
            .find(|(k, _)| *k == kind)
            .map_or(1, |(_, l)| *l);
        idx.sections.push(Section {
            kind: kind.to_owned(),
            level,
            title: squash(&masked[range]),
            starred: &c[2] == "*",
            line: loc.line(whole.start()),
        });
    }

    for c in re(&LABEL, r"\\label\s*\{([^}]*)\}").captures_iter(&masked) {
        let g = c.get(1).unwrap();
        let whole = c.get(0).unwrap();
        let line_no = loc.line(whole.start()) as usize;
        let context = match idx.sections.iter().find(|s| s.line as usize == line_no) {
            Some(s) => s.title.clone(),
            None => source
                .lines()
                .nth(line_no)
                .map(|l| squash(&l.replace(whole.as_str(), "")))
                .unwrap_or_default(),
        };
        idx.labels.push(Label {
            name: g.as_str().trim().to_owned(),
            span: loc.span(g.start(), g.end()),
            context: context.chars().take(120).collect(),
        });
    }

    let reference = re(
        &REF,
        r"\\(ref|eqref|autoref|cref|Cref|pageref|nameref|vref|labelcref|subref)\*?\s*(?:\[[^\]]*\])?\s*\{([^}]*)\}",
    );
    for c in reference.captures_iter(&masked) {
        let g = c.get(2).unwrap();
        for (name, from, to) in comma_items(g.as_str(), g.start()) {
            idx.references.push(Reference {
                name,
                command: c[1].to_owned(),
                span: loc.span(from, to),
            });
        }
    }

    let cite = re(
        &CITE,
        r"\\[A-Za-z]*cite[A-Za-z]*\*?\s*(?:\[[^\]]*\]\s*){0,2}\{([^}]*)\}",
    );
    for c in cite.captures_iter(&masked) {
        let g = c.get(1).unwrap();
        for (key, from, to) in comma_items(g.as_str(), g.start()) {
            idx.citations.push(Citation {
                key,
                span: loc.span(from, to),
            });
        }
    }

    let include = re(
        &INCLUDE,
        r"\\(input|include|subfile|includegraphics|bibliography|addbibresource)\*?\s*(?:\[[^\]]*\])?\s*\{([^}]*)\}",
    );
    for c in include.captures_iter(&masked) {
        let kind = match &c[1] {
            "input" => IncludeKind::Input,
            "include" => IncludeKind::Include,
            "subfile" => IncludeKind::Subfile,
            "includegraphics" => IncludeKind::Graphics,
            _ => IncludeKind::Bibliography,
        };
        let line = loc.line(c.get(0).unwrap().start());
        if &c[1] == "bibliography" {
            for p in c[2].split(',').map(str::trim).filter(|p| !p.is_empty()) {
                idx.includes.push(Include {
                    kind,
                    path: p.to_owned(),
                    line,
                });
            }
        } else {
            idx.includes.push(Include {
                kind,
                path: c[2].trim().to_owned(),
                line,
            });
        }
    }
    let import = re(&IMPORT, r"\\(?:sub)?import\*?\s*\{([^}]*)\}\s*\{([^}]*)\}");
    for c in import.captures_iter(&masked) {
        let dir = c[1].trim().trim_end_matches('/');
        idx.includes.push(Include {
            kind: IncludeKind::Import,
            path: format!("{dir}/{}", c[2].trim()),
            line: loc.line(c.get(0).unwrap().start()),
        });
    }

    let package = re(
        &PACKAGE,
        r"\\(?:usepackage|RequirePackage)\s*(?:\[([^\]]*)\])?\s*\{([^}]*)\}",
    );
    for c in package.captures_iter(&masked) {
        let options = squash(c.get(1).map_or("", |m| m.as_str()));
        for name in c[2].split(',').map(str::trim).filter(|n| !n.is_empty()) {
            idx.packages.push(Package {
                name: name.to_owned(),
                options: options.clone(),
            });
        }
    }

    let newcmd = re(
        &NEWCMD,
        r"\\(?:newcommand|renewcommand|providecommand|DeclareRobustCommand|NewDocumentCommand|DeclareDocumentCommand)\*?\s*\{?\s*\\([A-Za-z@]+)\s*\}?\s*(?:\[(\d)\])?",
    );
    for c in newcmd.captures_iter(&masked) {
        idx.commands.push(UserCommand {
            name: c[1].to_owned(),
            kind: UserCommandKind::Command,
            args: c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
            line: loc.line(c.get(0).unwrap().start()),
        });
    }
    for c in re(
        &MATHOP,
        r"\\DeclareMathOperator\*?\s*\{\s*\\([A-Za-z]+)\s*\}",
    )
    .captures_iter(&masked)
    {
        idx.commands.push(UserCommand {
            name: c[1].to_owned(),
            kind: UserCommandKind::Operator,
            args: 0,
            line: loc.line(c.get(0).unwrap().start()),
        });
    }
    for c in re(
        &NEWENV,
        r"\\(?:newenvironment|renewenvironment|NewDocumentEnvironment)\*?\s*\{([A-Za-z*]+)\}\s*(?:\[(\d)\])?",
    )
    .captures_iter(&masked)
    {
        idx.commands.push(UserCommand {
            name: c[1].to_owned(),
            kind: UserCommandKind::Environment,
            args: c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
            line: loc.line(c.get(0).unwrap().start()),
        });
    }

    // Floats (figure/table) com legenda e label.
    let float = re(&FLOAT, r"\\begin\{(figure|table)(\*?)\}");
    let caption = re(&CAPTION, r"\\caption\s*(?:\[[^\]]*\])?\s*\{");
    for c in float.captures_iter(&masked) {
        let whole = c.get(0).unwrap();
        let start = whole.end();
        let end_tag = format!("\\end{{{}{}}}", &c[1], &c[2]);
        let end = masked[start..]
            .find(&end_tag)
            .map_or(masked.len(), |p| start + p);
        let body = &masked[start..end];
        let caption_text = caption
            .find(body)
            .and_then(|m| read_group(body, m.end() - 1).map(|(r, _)| squash(&body[r])))
            .unwrap_or_default();
        let label = re(&FLOAT_LABEL, r"\\label\s*\{([^}]*)\}")
            .captures(body)
            .map(|l| l[1].trim().to_owned());
        // A prévia do label dentro do float é a legenda.
        if let (Some(name), false) = (&label, caption_text.is_empty()) {
            if let Some(l) = idx.labels.iter_mut().find(|l| &l.name == name) {
                l.context = caption_text.chars().take(120).collect();
            }
        }
        idx.floats.push(Float {
            kind: c[1].to_owned(),
            caption: caption_text,
            label,
            line: loc.line(whole.start()),
        });
    }

    // Labels sem texto na linha usam o título da seção anterior como contexto.
    for l in &mut idx.labels {
        if l.context.is_empty() {
            if let Some(s) = idx.sections.iter().rev().find(|s| s.line <= l.span.line) {
                l.context = s.title.clone();
            }
        }
    }
    idx
}

// ───────────────────────── projeto ─────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub index: FileIndex,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub main: Option<String>,
    /// Arquivos `.tex` na ordem em que aparecem na árvore a partir do principal.
    pub files: Vec<FileEntry>,
    /// Inclusões `\input` etc. que não existem no disco.
    pub missing: Vec<String>,
}

/// Normaliza `.`/`..` sem tocar o disco.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn key(path: &Path) -> String {
    normalize(path).to_string_lossy().into_owned()
}

#[derive(Default)]
pub struct Indexer {
    files: HashMap<String, FileIndex>,
}

impl Indexer {
    /// Reanalisa só um arquivo (atualização incremental).
    pub fn update(&mut self, path: &Path, text: &str) {
        self.files.insert(key(path), parse(text));
    }

    pub fn remove(&mut self, path: &Path) {
        self.files.remove(&key(path));
    }

    /// Indexa todos os `.tex` da pasta (respeitando `.gitignore`).
    pub fn scan(&mut self, root: &Path) {
        self.files.clear();
        for entry in ignore::WalkBuilder::new(root).build().flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "tex") && p.is_file() {
                if let Ok(bytes) = std::fs::read(p) {
                    self.update(p, &textfile::decode(&bytes).text);
                }
            }
        }
    }

    fn resolve_include(&self, base: &Path, raw: &str) -> Option<PathBuf> {
        [
            normalize(&base.join(raw)),
            normalize(&base.join(format!("{raw}.tex"))),
        ]
        .into_iter()
        .find(|p| self.files.contains_key(&key(p)))
    }

    /// Escolhe o arquivo principal: magic comment > manifesto > `\documentclass` não incluído.
    pub fn detect_main(
        &self,
        root: &Path,
        manifest_main: Option<&str>,
        hint: Option<&Path>,
    ) -> Option<PathBuf> {
        if let Some(h) = hint {
            if let Some(root_ref) = self.files.get(&key(h)).and_then(|f| f.magic_root.clone()) {
                let cand = normalize(&h.parent().unwrap_or(root).join(root_ref.trim()));
                if self.files.contains_key(&key(&cand)) {
                    return Some(cand);
                }
            }
        }
        if let Some(m) = manifest_main {
            let cand = normalize(&root.join(m));
            if self.files.contains_key(&key(&cand)) {
                return Some(cand);
            }
        }
        let mut included: HashSet<String> = HashSet::new();
        for (path, f) in &self.files {
            let own_dir = Path::new(path).parent().unwrap_or(root);
            for inc in &f.includes {
                if matches!(inc.kind, IncludeKind::Graphics | IncludeKind::Bibliography) {
                    continue;
                }
                for base in [root, own_dir] {
                    if let Some(r) = self.resolve_include(base, &inc.path) {
                        included.insert(key(&r));
                    }
                }
            }
        }
        let mut candidates: Vec<&String> = self
            .files
            .iter()
            .filter(|(_, f)| f.document_class.as_deref().is_some_and(|c| c != "subfiles"))
            .map(|(p, _)| p)
            .collect();
        candidates.sort();
        let is_main = |p: &&String| {
            Path::new(p.as_str())
                .file_stem()
                .is_some_and(|s| s == "main")
        };
        candidates
            .iter()
            .copied()
            .find(|p| !included.contains(*p) && is_main(p))
            .or_else(|| candidates.iter().copied().find(|p| !included.contains(*p)))
            .or_else(|| candidates.first().copied())
            .map(PathBuf::from)
    }

    /// Resolve a árvore de inclusões a partir de `main`.
    pub fn view(&self, root: &Path, main: Option<&Path>) -> ProjectView {
        let mut view = ProjectView {
            main: main.map(key),
            files: Vec::new(),
            missing: Vec::new(),
        };
        let Some(main) = main else { return view };
        let base = main.parent().unwrap_or(root).to_path_buf();
        let mut seen = HashSet::new();
        self.walk(&base, &normalize(main), &mut seen, &mut view);
        view
    }

    fn walk(&self, base: &Path, file: &Path, seen: &mut HashSet<String>, view: &mut ProjectView) {
        let k = key(file);
        if !seen.insert(k.clone()) {
            return;
        }
        let Some(index) = self.files.get(&k) else {
            return;
        };
        view.files.push(FileEntry {
            path: k,
            index: index.clone(),
        });
        for inc in &index.includes {
            if matches!(inc.kind, IncludeKind::Graphics | IncludeKind::Bibliography) {
                continue;
            }
            match self.resolve_include(base, &inc.path) {
                Some(next) => self.walk(base, &next, seen, view),
                None => view.missing.push(inc.path.clone()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrai_secoes_labels_e_refs_com_posicoes() {
        let src =
            "\\section{Introdução}\\label{sec:intro}\nVeja \\ref{sec:intro} e \\cref{a, b}.\n";
        let i = parse(src);
        assert_eq!(i.sections.len(), 1);
        assert_eq!(i.sections[0].title, "Introdução");
        assert_eq!(i.sections[0].level, 1);
        assert_eq!(i.labels[0].name, "sec:intro");
        assert_eq!(
            i.labels[0].span,
            Span {
                line: 0,
                col: 27,
                end_col: 36
            }
        );
        assert_eq!(i.labels[0].context, "Introdução");
        let names: Vec<_> = i.references.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["sec:intro", "a", "b"]);
        assert_eq!(
            i.references[2].span,
            Span {
                line: 1,
                col: 32,
                end_col: 33
            }
        );
    }

    #[test]
    fn colunas_sao_utf16() {
        let i = parse("é😀 \\label{x}");
        // 'é' = 1, '😀' = 2 unidades UTF-16, espaço = 1, "\label{" = 7
        assert_eq!(i.labels[0].span.col, 11);
    }

    #[test]
    fn ignora_comentarios_e_verbatim() {
        let src = "% \\label{a}\n\\begin{verbatim}\n\\label{b}\n\\end{verbatim}\n50\\% \\label{c} % \\label{d}\n";
        let names: Vec<_> = parse(src).labels.into_iter().map(|l| l.name).collect();
        assert_eq!(names, ["c"]);
    }

    #[test]
    fn floats_com_legenda_viram_contexto_do_label() {
        let src = "\\begin{figure}\n\\caption{Um gráfico}\n\\label{fig:g}\n\\end{figure}\n";
        let i = parse(src);
        assert_eq!(i.floats[0].caption, "Um gráfico");
        assert_eq!(i.floats[0].label.as_deref(), Some("fig:g"));
        assert_eq!(i.labels[0].context, "Um gráfico");
    }

    #[test]
    fn comandos_do_usuario_pacotes_e_citacoes() {
        let src = "\\documentclass[12pt]{abntex2}\n\\usepackage[utf8]{inputenc}\n\\usepackage{amsmath, graphicx}\n\\newcommand{\\R}{\\mathbb{R}}\n\\newcommand\\vet[1]{#1}\n\\DeclareMathOperator{\\sen}{sen}\n\\newenvironment{caixa}[1]{}{}\n\\cite[p.~2]{k1,k2} \\textcite{k3}\n";
        let i = parse(src);
        assert_eq!(i.document_class.as_deref(), Some("abntex2"));
        let pk: Vec<_> = i.packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(pk, ["inputenc", "amsmath", "graphicx"]);
        assert_eq!(i.packages[0].options, "utf8");
        let cmds: Vec<_> = i
            .commands
            .iter()
            .map(|c| (c.name.as_str(), c.kind, c.args))
            .collect();
        assert_eq!(
            cmds,
            [
                ("R", UserCommandKind::Command, 0),
                ("vet", UserCommandKind::Command, 1),
                ("sen", UserCommandKind::Operator, 0),
                ("caixa", UserCommandKind::Environment, 1),
            ]
        );
        let keys: Vec<_> = i.citations.iter().map(|c| c.key.as_str()).collect();
        assert_eq!(keys, ["k1", "k2", "k3"]);
    }

    #[test]
    fn inclusoes_e_magic_comment() {
        let src = "% !TEX root = ../main.tex\n\\input{cap1}\\include{cap2}\\import{sub/}{x}\\bibliography{a,b}\\includegraphics[width=1cm]{img/f.png}\n";
        let i = parse(src);
        assert_eq!(i.magic_root.as_deref(), Some("../main.tex"));
        let inc: Vec<_> = i
            .includes
            .iter()
            .map(|x| (x.kind, x.path.as_str()))
            .collect();
        assert_eq!(
            inc,
            [
                (IncludeKind::Input, "cap1"),
                (IncludeKind::Include, "cap2"),
                (IncludeKind::Bibliography, "a"),
                (IncludeKind::Bibliography, "b"),
                (IncludeKind::Graphics, "img/f.png"),
                (IncludeKind::Import, "sub/x"),
            ]
        );
    }

    #[test]
    fn tolera_chaves_sem_fechar() {
        let i = parse("\\section{Sem fim\n\\label{ok}");
        assert!(i.sections.is_empty());
        assert_eq!(i.labels.len(), 1);
    }

    fn write(dir: &Path, rel: &str, content: &str) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn resolve_arvore_a_partir_do_principal() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "main.tex",
            "\\documentclass{article}\n\\input{chapters/a}\n\\include{b}\n\\input{nao_existe}\n",
        );
        write(d.path(), "chapters/a.tex", "\\section{A}\\input{b}\n");
        write(d.path(), "b.tex", "\\section{B}\\input{main}\n");
        write(d.path(), "solto.tex", "\\section{Solto}\n");
        let mut ix = Indexer::default();
        ix.scan(d.path());
        let main = ix.detect_main(d.path(), None, None).unwrap();
        assert!(main.ends_with("main.tex"));
        let v = ix.view(d.path(), Some(&main));
        let names: Vec<_> = v
            .files
            .iter()
            .map(|f| {
                Path::new(&f.path)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, ["main.tex", "a.tex", "b.tex"]);
        assert_eq!(v.missing, ["nao_existe"]);
    }

    #[test]
    fn magic_comment_e_manifesto_tem_prioridade() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "a.tex", "\\documentclass{article}\n");
        write(d.path(), "b.tex", "\\documentclass{article}\n");
        write(d.path(), "sub/c.tex", "% !TEX root = ../b.tex\ntexto\n");
        let mut ix = Indexer::default();
        ix.scan(d.path());
        let hint = d.path().join("sub/c.tex");
        let m = ix
            .detect_main(d.path(), Some("a.tex"), Some(&hint))
            .unwrap();
        assert!(m.ends_with("b.tex"));
        let m = ix.detect_main(d.path(), Some("a.tex"), None).unwrap();
        assert!(m.ends_with("a.tex"));
    }

    #[test]
    fn atualizacao_incremental() {
        let d = tempfile::tempdir().unwrap();
        let mut ix = Indexer::default();
        let p = d.path().join("main.tex");
        ix.update(&p, "\\documentclass{article}\n");
        ix.update(&p, "\\documentclass{article}\\section{Novo}\n");
        let v = ix.view(d.path(), Some(&p));
        assert_eq!(v.files[0].index.sections[0].title, "Novo");
    }
}
