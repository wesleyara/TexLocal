//! Parser dos logs do LaTeX, BibTeX e Biber em diagnósticos estruturados (TL-404).
//!
//! O parser do LaTeX entende os dois formatos de erro (`arquivo:linha: mensagem` com
//! `-file-line-error` e o clássico `! mensagem` + `l.N`), reconstrói as linhas quebradas em 79
//! colunas, acompanha a pilha de arquivos abertos (parênteses) e classifica avisos de
//! referência, caixas (overfull/underfull), pacotes e fontes.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Latex,
    Reference,
    /// Overfull/underfull boxes: ocultos por padrão na interface.
    Box,
    Package,
    Font,
    Bibtex,
    Biber,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: Severity,
    pub category: Category,
    /// Caminho como aparece no log, normalizado (`/`, sem `./`). Relativo à raiz do projeto
    /// quando o arquivo pertence a ele.
    pub file: Option<String>,
    /// Linha (1-based).
    pub line: Option<u32>,
    pub message: String,
    /// Identificador da explicação amigável (chave `compile.explain.<id>` na interface).
    pub explanation: Option<&'static str>,
    /// Trecho de código informado pelo TeX (`l.12 ...`).
    pub context: Option<String>,
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("regex válida")
}

// ───────────────────────── explicações amigáveis ─────────────────────────

const EXPLANATIONS: &[(&str, &str)] = &[
    ("Undefined control sequence", "undefinedControlSequence"),
    ("Missing $ inserted", "missingDollar"),
    ("Missing { inserted", "missingBrace"),
    ("Missing } inserted", "missingBrace"),
    ("Extra }, or forgotten", "extraBrace"),
    ("Too many }'s", "tooManyBraces"),
    ("Runaway argument", "runaway"),
    ("Runaway definition", "runaway"),
    ("Paragraph ended before", "paragraphEnded"),
    ("Misplaced alignment tab", "misplacedAlignment"),
    ("Misplaced \\noalign", "misplacedAlignment"),
    ("Missing number", "missingNumber"),
    ("Illegal parameter number", "illegalParameter"),
    ("Emergency stop", "emergencyStop"),
    ("Fatal error occurred", "fatalError"),
    ("Environment", "undefinedEnvironment"),
    ("ended by \\end", "environmentMismatch"),
    ("Unicode character", "unicodeCharacter"),
    ("not found", "fileNotFound"),
    ("Can be used only in preamble", "preambleOnly"),
    ("Command \\", "commandAlreadyDefined"),
    ("Float(s) lost", "floatsLost"),
    ("Overfull \\hbox", "overfullBox"),
    ("Overfull \\vbox", "overfullBox"),
    ("Underfull \\hbox", "underfullBox"),
    ("Underfull \\vbox", "underfullBox"),
    ("There were undefined references", "undefinedReferences"),
    ("Label(s) may have changed", "rerunNeeded"),
    ("multiply defined", "multiplyDefined"),
    ("Font shape", "fontShape"),
    ("didn't find a database entry", "bibMissingEntry"),
    ("couldn't open database file", "bibMissingDatabase"),
    ("Cannot find", "bibMissingDatabase"),
    ("I found no \\citation", "bibNoCitations"),
    ("syntax error", "bibSyntax"),
    ("expecting", "bibSyntax"),
];

/// Mensagens de "Environment x undefined" e "File x not found" só entram com contexto próprio.
fn explain(message: &str, category: Category) -> Option<&'static str> {
    if let Some(rest) = message.strip_prefix("LaTeX Error: ") {
        if rest.starts_with("Environment") && rest.contains("undefined") {
            return Some("undefinedEnvironment");
        }
        if rest.starts_with("File") && rest.contains("not found") {
            return Some("fileNotFound");
        }
        if rest.contains("\\begin") && rest.contains("ended by") {
            return Some("environmentMismatch");
        }
    }
    if message.contains("Citation") && message.contains("undefined") {
        return Some("undefinedCitation");
    }
    if message.contains("Reference") && message.contains("undefined") {
        return Some("undefinedReference");
    }
    EXPLANATIONS
        .iter()
        .filter(|(needle, id)| {
            // "Environment", "not found" e "Command \" são genéricos: só valem para erros do
            // LaTeX, e "Environment" apenas junto de "undefined".
            match *id {
                "undefinedEnvironment" => {
                    message.contains("Environment") && message.contains("undefined")
                }
                "fileNotFound" => category == Category::Latex && message.contains("File"),
                "commandAlreadyDefined" => message.contains("already defined"),
                _ => message.contains(needle),
            }
        })
        .map(|(_, id)| *id)
        .next()
}

// ───────────────────────── LaTeX ─────────────────────────

static FILE_LINE_ERROR: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^(.+\.(?:tex|sty|cls|ltx|bbl|aux|def|cfg|clo|fd|ldf|dtx|toc|lof|lot|ind|gls|glo|out|lbx|cbx|bbx|bib|dbx|cnf)):(\d+): (.*)$",
    )
});
static BANG_ERROR: LazyLock<Regex> = LazyLock::new(|| re(r"^! (.*)$"));
static CONTEXT_LINE: LazyLock<Regex> = LazyLock::new(|| re(r"^l\.(\d+) ?(.*)$"));
static WARNING_HEADER: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(LaTeX(?: Font)? Warning|(?:Package|Class|Module) \S+ Warning): (.*)$"));
static CONTINUATION: LazyLock<Regex> = LazyLock::new(|| re(r"^\([^)\s/\\]+\)\s{2,}(.*)$"));
static INPUT_LINE: LazyLock<Regex> = LazyLock::new(|| re(r"\s*on input line (\d+)\.?\s*$"));
static BOX_LINES: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^((?:Overfull|Underfull) \\[hv]box \(.*?\)) (?:in paragraph |in alignment |in \S+ )?at lines? (\d+)(?:--\d+)?",
    )
});
static BOX_DETECTED: LazyLock<Regex> =
    LazyLock::new(|| re(r"^((?:Overfull|Underfull) \\[hv]box \(.*?\)) .*detected at line (\d+)"));
static BOX_OUTPUT: LazyLock<Regex> =
    LazyLock::new(|| re(r"^((?:Overfull|Underfull) \\[hv]box \(.*?\)) has occurred while"));
static MESSAGE_LINE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^(?:!|l\.\d|\([^)\s/\\]+\)\s{2,}|(?:Overfull|Underfull|Tight|Loose) |LaTeX (?:Font )?(?:Warning|Info|Error)|(?:Package|Class|Module) \S+ (?:Warning|Info|Error)|\\)",
    )
});
static WIN_ABSOLUTE: LazyLock<Regex> = LazyLock::new(|| re(r"^[A-Za-z]:[\\/]"));
static EXTENSION: LazyLock<Regex> = LazyLock::new(|| re(r"\.[A-Za-z0-9]{1,5}$"));

/// Limite de colunas do log do TeX (`max_print_line`): linhas com exatamente este tamanho
/// continuam na linha seguinte.
const PRINT_LINE: usize = 79;

/// Junta as linhas que o TeX quebrou em 79 colunas.
fn unwrap_lines(log: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut pending = String::new();
    for raw in log.lines() {
        let line = raw.trim_end_matches('\r');
        pending.push_str(line);
        if line.chars().count() == PRINT_LINE {
            continue;
        }
        out.push(std::mem::take(&mut pending));
    }
    if !pending.is_empty() {
        out.push(pending);
    }
    out
}

fn normalize_path(path: &str) -> String {
    let p = path.replace('\\', "/");
    p.strip_prefix("./").map(str::to_owned).unwrap_or(p)
}

fn is_absolute(path: &str) -> bool {
    path.starts_with('/') || WIN_ABSOLUTE.is_match(path)
}

fn looks_like_file(token: &str) -> bool {
    token.starts_with("./")
        || token.starts_with("../")
        || token.starts_with('/')
        || WIN_ABSOLUTE.is_match(token)
        || EXTENSION.is_match(token)
}

/// Pilha de arquivos abertos, atualizada pelos parênteses do log.
#[derive(Default)]
struct FileStack(Vec<Option<String>>);

impl FileStack {
    fn feed(&mut self, line: &str) {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            match chars[i] {
                '(' => {
                    let start = i + 1;
                    let mut end = start;
                    while end < chars.len() && !matches!(chars[end], ' ' | '(' | ')' | '\t') {
                        end += 1;
                    }
                    let token: String = chars[start..end].iter().collect();
                    self.0
                        .push(looks_like_file(&token).then(|| normalize_path(&token)));
                    i = end;
                }
                ')' => {
                    self.0.pop();
                    i += 1;
                }
                _ => i += 1,
            }
        }
    }

    /// Arquivo do usuário mais interno: prefere caminhos relativos (o projeto), pois arquivos
    /// de bibliotecas do TeX aparecem com caminho absoluto.
    fn current(&self) -> Option<String> {
        self.0
            .iter()
            .rev()
            .flatten()
            .find(|f| !is_absolute(f))
            .or_else(|| self.0.iter().rev().flatten().next())
            .cloned()
    }
}

fn warning_category(kind: &str, message: &str) -> Category {
    if kind.starts_with("LaTeX Font") {
        Category::Font
    } else if kind.starts_with("LaTeX") {
        let referencey = (message.contains("Reference") || message.contains("Citation"))
            && message.contains("undefined");
        if referencey
            || message.contains("undefined references")
            || message.contains("multiply defined")
            || message.contains("multiply-defined")
            || message.contains("Label(s) may have changed")
        {
            Category::Reference
        } else {
            Category::Latex
        }
    } else {
        Category::Package
    }
}

/// Lê um log do LaTeX (pdfTeX, XeTeX ou LuaTeX).
pub fn parse_latex_log(log: &str) -> Vec<Diagnostic> {
    let lines = unwrap_lines(log);
    let mut stack = FileStack::default();
    let mut out: Vec<Diagnostic> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].as_str();

        // Erro com `-file-line-error`.
        if let Some(c) = FILE_LINE_ERROR.captures(line) {
            let (context, next) = read_context(&lines, i + 1);
            let message = c[3].trim().to_owned();
            out.push(error(
                Some(normalize_path(&c[1])),
                c[2].parse().ok(),
                message,
                context,
            ));
            i = next;
            continue;
        }

        // Erro clássico: `! mensagem` ... `l.N código`.
        if let Some(c) = BANG_ERROR.captures(line) {
            let message = c[1].trim().to_owned();
            let (context, next) = read_context(&lines, i + 1);
            let ctx_line = context.as_ref().and_then(|(n, _)| *n);
            let line_no = ctx_line;
            out.push(error(stack.current(), line_no, message, context));
            i = next;
            continue;
        }

        // Avisos de LaTeX, pacotes, classes e fontes.
        if let Some(c) = WARNING_HEADER.captures(line) {
            let kind = c[1].to_owned();
            let mut message = c[2].trim().to_owned();
            let mut next = i + 1;
            while !INPUT_LINE.is_match(&message) && next < lines.len() {
                match CONTINUATION.captures(&lines[next]) {
                    Some(cont) => {
                        message.push(' ');
                        message.push_str(cont[1].trim());
                        next += 1;
                    }
                    None => break,
                }
            }
            let mut line_no = None;
            if let Some(m) = INPUT_LINE.captures(&message) {
                line_no = m[1].parse().ok();
                let whole = m.get(0).expect("grupo 0").start();
                message.truncate(whole);
            }
            let full = format!("{kind}: {message}");
            let category = warning_category(&kind, &message);
            out.push(Diagnostic {
                severity: Severity::Warning,
                category,
                file: line_no.and(stack.current()),
                line: line_no,
                explanation: explain(&full, category),
                message,
                context: None,
            });
            i = next;
            continue;
        }

        // Caixas overfull/underfull.
        if let Some(c) = BOX_LINES
            .captures(line)
            .or_else(|| BOX_DETECTED.captures(line))
        {
            out.push(box_warning(&c[1], c[2].parse().ok(), stack.current()));
            i += 1;
            continue;
        }
        if let Some(c) = BOX_OUTPUT.captures(line) {
            out.push(box_warning(&c[1], None, None));
            i += 1;
            continue;
        }

        if !MESSAGE_LINE.is_match(line) {
            stack.feed(line);
        }
        i += 1;
    }
    out
}

fn box_warning(head: &str, line: Option<u32>, file: Option<String>) -> Diagnostic {
    Diagnostic {
        severity: Severity::Warning,
        category: Category::Box,
        file: line.and(file),
        line,
        explanation: explain(head, Category::Box),
        message: head.to_owned(),
        context: None,
    }
}

fn error(
    file: Option<String>,
    line: Option<u32>,
    message: String,
    context: Option<(Option<u32>, String)>,
) -> Diagnostic {
    let explanation = explain(&message, Category::Latex);
    Diagnostic {
        severity: Severity::Error,
        category: Category::Latex,
        file,
        line: line.or_else(|| context.as_ref().and_then(|(n, _)| *n)),
        message,
        explanation,
        context: context.map(|(_, text)| text),
    }
}

/// Procura o trecho `l.N código` após um erro (até 25 linhas, parando em outro erro).
/// Devolve (linha, texto) e o índice a partir do qual continuar a leitura.
fn read_context(lines: &[String], from: usize) -> (Option<(Option<u32>, String)>, usize) {
    let mut j = from;
    while j < lines.len() && j < from + 25 {
        let l = lines[j].as_str();
        if FILE_LINE_ERROR.is_match(l) || BANG_ERROR.is_match(l) {
            break;
        }
        if let Some(c) = CONTEXT_LINE.captures(l) {
            let number = c[1].parse().ok();
            let mut text = format!("l.{} {}", &c[1], c[2].trim_end());
            let mut next = j + 1;
            // A linha seguinte traz o restante do trecho, depois do ponto do erro.
            if let Some(rest) = lines.get(next) {
                let rest = rest.trim();
                if !rest.is_empty()
                    && !FILE_LINE_ERROR.is_match(rest)
                    && !BANG_ERROR.is_match(rest)
                    && !rest.starts_with("LaTeX ")
                {
                    text.push(' ');
                    text.push_str(rest);
                    next += 1;
                }
            }
            return (Some((number, text.trim().to_owned())), next);
        }
        j += 1;
    }
    (None, from)
}

// ───────────────────────── BibTeX ─────────────────────────

static BIBTEX_WARNING: LazyLock<Regex> = LazyLock::new(|| re(r"^Warning--(.*)$"));
static BIBTEX_AT: LazyLock<Regex> = LazyLock::new(|| re(r"^(.*?)---line (\d+) of file (.+?)\s*$"));
static BIBTEX_WHILE: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(.*?)---while reading file (.+?)\s*$"));
static BIBTEX_OPEN: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(I couldn't open (?:database|style|auxiliary) file (.+?))\s*$"));

/// Lê o `.blg` do BibTeX.
pub fn parse_bibtex_log(log: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for raw in log.lines() {
        let line = raw.trim_end_matches('\r');
        let diag =
            |severity, file: Option<String>, line_no: Option<u32>, message: String| Diagnostic {
                severity,
                category: Category::Bibtex,
                file,
                line: line_no,
                explanation: explain(&message, Category::Bibtex),
                message,
                context: None,
            };
        if let Some(c) = BIBTEX_WARNING.captures(line) {
            out.push(diag(Severity::Warning, None, None, c[1].trim().to_owned()));
        } else if let Some(c) = BIBTEX_AT.captures(line) {
            out.push(diag(
                Severity::Error,
                Some(normalize_path(&c[3])),
                c[2].parse().ok(),
                c[1].trim().to_owned(),
            ));
        } else if let Some(c) = BIBTEX_WHILE.captures(line) {
            out.push(diag(
                Severity::Error,
                Some(normalize_path(&c[2])),
                None,
                c[1].trim().to_owned(),
            ));
        } else if let Some(c) = BIBTEX_OPEN.captures(line) {
            out.push(diag(
                Severity::Error,
                Some(normalize_path(&c[2])),
                None,
                c[1].to_owned(),
            ));
        } else if line.starts_with("Sorry---you've exceeded BibTeX's") {
            out.push(diag(Severity::Error, None, None, line.trim().to_owned()));
        }
    }
    out
}

// ───────────────────────── Biber ─────────────────────────

static BIBER_LINE: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(?:\[\d+\]\s+\S+?:\d+>\s+)?(WARN|ERROR|FATAL)\s+-\s+(.*)$"));
static BIBER_BIB_FILE: LazyLock<Regex> =
    LazyLock::new(|| re(r"([^\s,'/\\]+\.bib)(?:_[A-Za-z0-9]+\.utf8)?"));
static BIBER_LINE_NO: LazyLock<Regex> = LazyLock::new(|| re(r",\s*line (\d+)"));
static BIBER_CANNOT_FIND: LazyLock<Regex> = LazyLock::new(|| re(r"Cannot find '([^']+)'"));

/// Lê o `.blg` do Biber.
pub fn parse_biber_log(log: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for raw in log.lines() {
        let line = raw.trim_end_matches('\r');
        let Some(c) = BIBER_LINE.captures(line) else {
            continue;
        };
        let severity = if &c[1] == "WARN" {
            Severity::Warning
        } else {
            Severity::Error
        };
        let message = c[2].trim().to_owned();
        let file = BIBER_CANNOT_FIND
            .captures(&message)
            .map(|m| normalize_path(&m[1]))
            .or_else(|| {
                BIBER_BIB_FILE
                    .captures(&message)
                    .map(|m| normalize_path(&m[1]))
            });
        let line_no = BIBER_LINE_NO
            .captures(&message)
            .and_then(|m| m[1].parse().ok());
        out.push(Diagnostic {
            severity,
            category: Category::Biber,
            file,
            line: line_no,
            explanation: explain(&message, Category::Biber),
            message,
            context: None,
        });
    }
    out
}

/// Lê um `.blg` detectando se foi gerado pelo Biber ou pelo BibTeX.
pub fn parse_bib_log(log: &str) -> Vec<Diagnostic> {
    if log.contains("This is Biber")
        || BIBER_LINE.is_match(log.lines().next().unwrap_or(""))
        || log.lines().any(|l| l.contains("> INFO - "))
    {
        parse_biber_log(log)
    } else {
        parse_bibtex_log(log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LATEX_LOG: &str = include_str!("../tests/logs/pdftex-file-line-error.log");
    const CLASSIC_LOG: &str = include_str!("../tests/logs/pdftex-classic.log");
    const BIBTEX_LOG: &str = include_str!("../tests/logs/bibtex.blg");
    const BIBER_LOG: &str = include_str!("../tests/logs/biber.blg");

    fn find<'a>(d: &'a [Diagnostic], needle: &str) -> &'a Diagnostic {
        d.iter()
            .find(|x| x.message.contains(needle))
            .unwrap_or_else(|| panic!("diagnóstico {needle:?} não encontrado em {d:#?}"))
    }

    #[test]
    fn file_line_error_with_context() {
        let d = parse_latex_log(LATEX_LOG);
        let e = find(&d, "Undefined control sequence");
        assert_eq!(e.severity, Severity::Error);
        assert_eq!(e.file.as_deref(), Some("main.tex"));
        assert_eq!(e.line, Some(8));
        assert_eq!(e.explanation, Some("undefinedControlSequence"));
        assert!(e.context.as_deref().unwrap().contains("\\foo"));
    }

    #[test]
    fn error_in_included_file_keeps_its_path() {
        let d = parse_latex_log(LATEX_LOG);
        let e = find(&d, "Environment foo undefined");
        assert_eq!(e.file.as_deref(), Some("chapters/intro.tex"));
        assert_eq!(e.line, Some(3));
        assert_eq!(e.explanation, Some("undefinedEnvironment"));
    }

    #[test]
    fn missing_package_is_reported_as_file_not_found() {
        let d = parse_latex_log(LATEX_LOG);
        let e = find(&d, "File `nonexistent.sty' not found");
        assert_eq!(e.explanation, Some("fileNotFound"));
        assert_eq!(e.file.as_deref(), Some("main.tex"));
    }

    #[test]
    fn undefined_references_and_citations() {
        let d = parse_latex_log(LATEX_LOG);
        let r = find(&d, "Reference `fig:missing'");
        assert_eq!(r.severity, Severity::Warning);
        assert_eq!(r.category, Category::Reference);
        assert_eq!(r.line, Some(12));
        assert_eq!(r.file.as_deref(), Some("main.tex"));
        assert_eq!(r.explanation, Some("undefinedReference"));
        let c = find(&d, "Citation `knuth84'");
        assert_eq!(c.explanation, Some("undefinedCitation"));
        assert_eq!(c.line, Some(14));
        let summary = find(&d, "There were undefined references");
        assert_eq!(summary.category, Category::Reference);
        assert_eq!(summary.line, None);
    }

    #[test]
    fn warning_attributed_to_file_inside_stack() {
        let d = parse_latex_log(LATEX_LOG);
        let w = find(&d, "Reference `sec:later'");
        assert_eq!(w.file.as_deref(), Some("chapters/intro.tex"));
        assert_eq!(w.line, Some(5));
    }

    #[test]
    fn boxes_are_classified_and_located() {
        let d = parse_latex_log(LATEX_LOG);
        let over = find(&d, "Overfull \\hbox (12.5pt too wide)");
        assert_eq!(over.category, Category::Box);
        assert_eq!(over.line, Some(20));
        assert_eq!(over.file.as_deref(), Some("main.tex"));
        let under = find(&d, "Underfull \\hbox (badness 10000)");
        assert_eq!(under.line, Some(31));
        assert_eq!(under.explanation, Some("underfullBox"));
    }

    #[test]
    fn multiline_package_warning_is_joined() {
        let d = parse_latex_log(LATEX_LOG);
        let w = find(&d, "Token not allowed in a PDF string");
        assert_eq!(w.category, Category::Package);
        assert_eq!(w.line, Some(18));
        assert!(w.message.contains("removing"));
    }

    #[test]
    fn library_files_do_not_hijack_the_location() {
        let d = parse_latex_log(LATEX_LOG);
        let w = find(&d, "Font shape `OT1/cmr/m/n' in size <3.5> not available");
        assert_eq!(w.category, Category::Font);
        assert_eq!(w.file.as_deref(), Some("main.tex"));
    }

    #[test]
    fn wrapped_lines_are_rejoined() {
        let d = parse_latex_log(LATEX_LOG);
        // A mensagem original passa de 79 colunas e foi quebrada pelo TeX.
        let w = find(
            &d,
            "Citation `very-long-bibliography-key-from-a-large-database-2024'",
        );
        assert_eq!(w.line, Some(22));
    }

    #[test]
    fn classic_bang_errors_use_the_file_stack() {
        let d = parse_latex_log(CLASSIC_LOG);
        let e = find(&d, "Missing $ inserted");
        assert_eq!(e.file.as_deref(), Some("chapters/math.tex"));
        assert_eq!(e.line, Some(7));
        assert_eq!(e.explanation, Some("missingDollar"));
        let fatal = find(&d, "Emergency stop");
        assert_eq!(fatal.file.as_deref(), Some("main.tex"));
        assert_eq!(fatal.explanation, Some("emergencyStop"));
    }

    #[test]
    fn clean_log_has_no_diagnostics() {
        let log = "This is pdfTeX\n(./main.tex\n(/usr/share/texlive/article.cls\nDocument Class: article)\n[1{/var/lib/fonts.map}] )\nOutput written on main.pdf (1 page).\n";
        assert!(parse_latex_log(log).is_empty());
    }

    #[test]
    fn bibtex_log() {
        let d = parse_bibtex_log(BIBTEX_LOG);
        let missing = find(&d, "I didn't find a database entry for \"lamport94\"");
        assert_eq!(missing.severity, Severity::Warning);
        assert_eq!(missing.category, Category::Bibtex);
        assert_eq!(missing.explanation, Some("bibMissingEntry"));
        let syntax = find(&d, "I was expecting a `,' or a `}'");
        assert_eq!(syntax.severity, Severity::Error);
        assert_eq!(syntax.file.as_deref(), Some("refs.bib"));
        assert_eq!(syntax.line, Some(12));
        let open = find(&d, "I couldn't open database file extra.bib");
        assert_eq!(open.file.as_deref(), Some("extra.bib"));
    }

    #[test]
    fn biber_log() {
        let d = parse_biber_log(BIBER_LOG);
        let syntax = find(&d, "syntax error");
        assert_eq!(syntax.severity, Severity::Error);
        assert_eq!(syntax.file.as_deref(), Some("refs.bib"));
        assert_eq!(syntax.line, Some(5));
        let warn = find(&d, "Entry 'doe2020'");
        assert_eq!(warn.severity, Severity::Warning);
        let cannot = find(&d, "Cannot find 'missing.bib'");
        assert_eq!(cannot.file.as_deref(), Some("missing.bib"));
        assert_eq!(cannot.explanation, Some("bibMissingDatabase"));
    }

    #[test]
    fn bib_log_autodetects_the_tool() {
        assert!(parse_bib_log(BIBER_LOG)
            .iter()
            .all(|d| d.category == Category::Biber));
        assert!(parse_bib_log(BIBTEX_LOG)
            .iter()
            .all(|d| d.category == Category::Bibtex));
    }

    #[test]
    fn windows_line_endings_and_paths() {
        let log =
            "(.\\main.tex\r\n./main.tex:3: Undefined control sequence.\r\nl.3 \\oops\r\n\r\n)\r\n";
        let d = parse_latex_log(log);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].file.as_deref(), Some("main.tex"));
        assert_eq!(d[0].line, Some(3));
    }
}
