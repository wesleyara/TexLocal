//! Extração do texto "de prosa" de um documento LaTeX: palavras fora de comandos, math,
//! verbatim, comentários e argumentos que não são texto (`\ref`, `\cite`, `\usepackage`...).
//! Compartilhado pela contagem de palavras (TL-310) e pelo corretor ortográfico (TL-309).

/// Comandos cujos argumentos (`[..]` e `{..}` seguintes) não são texto.
const SKIP_ARGS: &[&str] = &[
    "ref",
    "eqref",
    "autoref",
    "cref",
    "Cref",
    "pageref",
    "nameref",
    "vref",
    "label",
    "cite",
    "citep",
    "citet",
    "citeauthor",
    "citeyear",
    "nocite",
    "textcite",
    "parencite",
    "autocite",
    "footcite",
    "usepackage",
    "RequirePackage",
    "documentclass",
    "input",
    "include",
    "subfile",
    "includegraphics",
    "bibliography",
    "bibliographystyle",
    "addbibresource",
    "url",
    "href",
    "newcommand",
    "renewcommand",
    "providecommand",
    "DeclareMathOperator",
    "newenvironment",
    "renewenvironment",
    "def",
    "setlength",
    "addtolength",
    "vspace",
    "hspace",
    "pagestyle",
    "thispagestyle",
    "newcounter",
    "setcounter",
    "addcounter",
    "usetikzlibrary",
    "geometry",
    "hypersetup",
    "graphicspath",
    "lstset",
    "newtheorem",
    "theoremstyle",
    "color",
    "textcolor",
    "colorbox",
    "definecolor",
    "babel",
    "selectlanguage",
    "foreignlanguage",
    "linewidth",
    "renewcommand*",
    "newcommand*",
    "importpackage",
    "bibitem",
    "glsadd",
    "gls",
    "acrshort",
    "acrlong",
    "acrfull",
    "index",
    "includepdf",
    "lstinputlisting",
    "inputminted",
    "mintinline",
];

const MATH_ENVS: &[&str] = &[
    "equation",
    "align",
    "alignat",
    "gather",
    "multline",
    "eqnarray",
    "displaymath",
    "math",
    "flalign",
    "split",
    "array",
    "matrix",
    "pmatrix",
    "bmatrix",
    "vmatrix",
    "cases",
    "tikzpicture",
    "tabular",
    "tabularx",
    "longtable",
    "thebibliography",
];

const VERBATIM_ENVS: &[&str] = &[
    "verbatim",
    "Verbatim",
    "lstlisting",
    "minted",
    "comment",
    "filecontents",
    "luacode",
];

pub const SECTIONS: &[(&str, i8)] = &[
    ("part", -1),
    ("chapter", 0),
    ("section", 1),
    ("subsection", 2),
    ("subsubsection", 3),
    ("paragraph", 4),
    ("subparagraph", 5),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    /// Offsets em bytes no texto original.
    pub start: usize,
    pub end: usize,
    /// Faz parte do título de uma seção.
    pub header: bool,
    /// Colada a um comando de acento (`\'e`) — o corretor ignora para evitar falsos positivos.
    pub touches_escape: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub start: usize,
    pub level: i8,
    pub title: String,
}

#[derive(Debug, Default)]
pub struct Scan {
    pub tokens: Vec<Token>,
    pub headings: Vec<Heading>,
    pub inline_math: usize,
    pub display_math: usize,
}

struct Scanner<'a> {
    text: &'a str,
    chars: Vec<(usize, char)>,
    out: Scan,
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '’' || c == '-'
}

impl<'a> Scanner<'a> {
    fn byte(&self, i: usize) -> usize {
        self.chars.get(i).map_or(self.text.len(), |c| c.0)
    }
    fn at(&self, i: usize) -> Option<char> {
        self.chars.get(i).map(|c| c.1)
    }
    /// Índice do primeiro caractere em ou após o byte `b`.
    fn index_of_byte(&self, b: usize) -> usize {
        self.chars.partition_point(|c| c.0 < b)
    }
    /// Índice logo após a próxima ocorrência de `pat` a partir de `i` (ou o fim).
    fn after(&self, i: usize, pat: &str) -> usize {
        let from = self.byte(i);
        match self.text[from..].find(pat) {
            Some(p) => self.index_of_byte(from + p + pat.len()),
            None => self.chars.len(),
        }
    }
    fn skip_ws(&self, mut i: usize) -> usize {
        while self.at(i).is_some_and(char::is_whitespace) {
            i += 1;
        }
        i
    }
    /// `{...}` balanceado a partir de `i` (que deve ser `{`). Devolve (índice interno inicial, índice após `}`).
    fn group(&self, i: usize) -> Option<(usize, usize)> {
        if self.at(i) != Some('{') {
            return None;
        }
        let mut depth = 0;
        let mut j = i;
        while let Some(c) = self.at(j) {
            match c {
                '\\' => j += 1,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((i + 1, j + 1));
                    }
                }
                _ => {}
            }
            j += 1;
        }
        None
    }
    fn optional(&self, i: usize) -> Option<usize> {
        if self.at(i) != Some('[') {
            return None;
        }
        let mut j = i;
        while let Some(c) = self.at(j) {
            match c {
                '\\' => j += 1,
                ']' => return Some(j + 1),
                '\n' if self.at(j + 1) == Some('\n') => return None,
                _ => {}
            }
            j += 1;
        }
        None
    }
    /// Pula `[..]` e `{..}` consecutivos (no máximo `groups` grupos obrigatórios).
    fn skip_args(&self, mut i: usize, groups: usize) -> usize {
        let mut taken = 0;
        loop {
            let j = self.skip_ws(i);
            if let Some(n) = self.optional(j) {
                i = n;
            } else if taken < groups {
                match self.group(j) {
                    Some((_, n)) => {
                        i = n;
                        taken += 1;
                    }
                    None => return i,
                }
            } else {
                return i;
            }
        }
    }

    fn env_name(&self, i: usize) -> Option<(String, usize)> {
        let (a, b) = self.group(self.skip_ws(i))?;
        let name: String = self.chars[a..b - 1].iter().map(|c| c.1).collect();
        Some((name.trim().to_owned(), b))
    }

    fn run(mut self) -> Scan {
        let mut start = 0;
        let mut end = self.chars.len();
        if let Some(p) = self.text.find("\\begin{document}") {
            start = self.index_of_byte(p + "\\begin{document}".len());
        }
        if let Some(p) = self.text.rfind("\\end{document}") {
            end = self.index_of_byte(p);
        }
        self.scan(start, end, false);
        self.out
    }

    fn scan(&mut self, from: usize, to: usize, header: bool) {
        let mut i = from;
        // palavra em construção: (índice inicial, tocou escape)
        let mut word: Option<(usize, bool)> = None;
        let mut after_escape = false;
        macro_rules! flush {
            ($i:expr) => {
                if let Some((s, esc)) = word.take() {
                    self.push_word(s, $i, header, esc);
                }
            };
        }
        while i < to {
            let c = self.chars[i].1;
            if is_word_char(c) {
                if word.is_none() {
                    word = Some((i, after_escape));
                }
                after_escape = false;
                i += 1;
                continue;
            }
            // qualquer outro caractere encerra a palavra
            let esc_next = c == '\\';
            flush!(i);
            after_escape = false;
            match c {
                '%' => i = self.after(i, "\n").min(to),
                '$' => {
                    if self.at(i + 1) == Some('$') {
                        self.out.display_math += 1;
                        i = self.after(i + 2, "$$");
                    } else {
                        self.out.inline_math += 1;
                        i = self.skip_inline_math(i + 1);
                    }
                }
                '\\' if esc_next => {
                    i = self.command(i, to);
                    after_escape = true;
                    // palavra colada ao comando de acento: marca como "toca escape"
                    if let Some((s, _)) = word {
                        word = Some((s, true));
                    }
                }
                _ => i += 1,
            }
        }
        flush!(to.min(self.chars.len()));
    }

    fn skip_inline_math(&self, mut i: usize) -> usize {
        while let Some(c) = self.at(i) {
            match c {
                '\\' => i += 1,
                '$' => return i + 1,
                _ => {}
            }
            i += 1;
        }
        i
    }

    /// Trata um comando que começa em `i` (uma barra invertida). Devolve o próximo índice.
    fn command(&mut self, i: usize, to: usize) -> usize {
        let Some(n) = self.at(i + 1) else {
            return i + 1;
        };
        match n {
            '(' => {
                self.out.inline_math += 1;
                return self.after(i + 2, "\\)");
            }
            '[' => {
                self.out.display_math += 1;
                return self.after(i + 2, "\\]");
            }
            c if !c.is_ascii_alphabetic() && c != '@' => return i + 2,
            _ => {}
        }
        let mut j = i + 1;
        while self
            .at(j)
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '@')
        {
            j += 1;
        }
        let name: String = self.chars[i + 1..j].iter().map(|c| c.1).collect();
        if self.at(j) == Some('*') {
            j += 1;
        }
        match name.as_str() {
            "begin" | "end" => {
                let Some((env, next)) = self.env_name(j) else {
                    return j;
                };
                if name == "begin" {
                    if MATH_ENVS.contains(&env.trim_end_matches('*')) {
                        self.out.display_math += 1;
                        return self.after(next, &format!("\\end{{{env}}}"));
                    }
                    if VERBATIM_ENVS.contains(&env.as_str()) {
                        return self.after(next, &format!("\\end{{{env}}}"));
                    }
                    return self.skip_args_once_optional(next);
                }
                next
            }
            "verb" => match self.at(j) {
                Some(d) => self.after(j + 1, &d.to_string()),
                None => j,
            },
            n if SECTIONS.iter().any(|(k, _)| *k == n) => {
                let level = SECTIONS.iter().find(|(k, _)| *k == n).map_or(1, |s| s.1);
                let after_opt = self.skip_optional_only(j);
                let g = self.group(self.skip_ws(after_opt));
                let title = g
                    .map(|(a, b)| self.chars[a..b - 1].iter().map(|c| c.1).collect::<String>())
                    .unwrap_or_default();
                self.out.headings.push(Heading {
                    start: self.byte(i),
                    level,
                    title: title.split_whitespace().collect::<Vec<_>>().join(" "),
                });
                match g {
                    Some((a, b)) => {
                        self.scan(a, b - 1, true);
                        b
                    }
                    None => after_opt,
                }
            }
            n if SKIP_ARGS.contains(&n) => {
                let groups = match n {
                    "href" | "newtheorem" | "definecolor" | "textcolor" | "colorbox" => 2,
                    "foreignlanguage" => 1,
                    _ => 1,
                };
                self.skip_args(j, groups).min(to.max(j))
            }
            _ => j,
        }
    }

    fn skip_optional_only(&self, i: usize) -> usize {
        let j = self.skip_ws(i);
        self.optional(j).unwrap_or(i)
    }

    fn skip_args_once_optional(&self, i: usize) -> usize {
        let j = self.skip_ws(i);
        if self.at(j) == Some('[') {
            self.optional(j).unwrap_or(i)
        } else {
            i
        }
    }

    fn push_word(&mut self, s: usize, e: usize, header: bool, touches_escape: bool) {
        let raw: String = self.chars[s..e].iter().map(|c| c.1).collect();
        let trimmed_start = raw.len() - raw.trim_start_matches(['\'', '’', '-']).len();
        let trimmed = raw.trim_matches(['\'', '’', '-']);
        if !trimmed.chars().any(char::is_alphanumeric) {
            return;
        }
        let start = self.byte(s) + trimmed_start;
        self.out.tokens.push(Token {
            text: trimmed.to_owned(),
            start,
            end: start + trimmed.len(),
            header,
            touches_escape: touches_escape || self.touches_backslash(e),
        });
    }

    fn touches_backslash(&self, e: usize) -> bool {
        self.at(e) == Some('\\') && self.at(e + 1).is_some_and(|c| "'\"^~`=.".contains(c))
    }
}

/// Varre o texto e devolve palavras, títulos e contagem de math.
pub fn scan(text: &str) -> Scan {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let scanner = Scanner {
        text,
        chars,
        out: Scan::default(),
    };
    scanner.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(src: &str) -> Vec<String> {
        scan(src).tokens.into_iter().map(|t| t.text).collect()
    }

    #[test]
    fn ignora_comandos_math_e_comentarios() {
        let src = "Olá \\textbf{mundo} com $x+y$ e \\ref{fig:a} % nada aqui\nfim \\cite{k}.";
        assert_eq!(words(src), ["Olá", "mundo", "com", "e", "fim"]);
    }

    #[test]
    fn ignora_ambientes_de_math_e_verbatim() {
        let src = "antes\n\\begin{equation}\nx = y\n\\end{equation}\n\\begin{verbatim}\ncodigo aqui\n\\end{verbatim}\ndepois";
        assert_eq!(words(src), ["antes", "depois"]);
    }

    #[test]
    fn preambulo_nao_conta() {
        let src = "\\documentclass{article}\n\\title{Meu titulo}\n\\begin{document}\nconta isto\n\\end{document}\nnao isto";
        assert_eq!(words(src), ["conta", "isto"]);
    }

    #[test]
    fn titulos_de_secao_sao_cabecalhos() {
        let s = scan("\\section{Uma seção}\ntexto\n");
        assert_eq!(s.headings.len(), 1);
        assert_eq!(s.headings[0].title, "Uma seção");
        let headers: Vec<_> = s
            .tokens
            .iter()
            .filter(|t| t.header)
            .map(|t| &t.text[..])
            .collect();
        assert_eq!(headers, ["Uma", "seção"]);
        assert_eq!(s.tokens.iter().filter(|t| !t.header).count(), 1);
    }

    #[test]
    fn hifen_e_apostrofo_ficam_na_palavra() {
        assert_eq!(
            words("bem-vindo don't 'aspas' --"),
            ["bem-vindo", "don't", "aspas"]
        );
    }

    #[test]
    fn offsets_apontam_para_o_original() {
        let src = "ação \\emph{rápida}";
        let t = scan(src).tokens;
        assert_eq!(&src[t[1].start..t[1].end], "rápida");
    }

    #[test]
    fn argumentos_opcionais_e_obrigatorios_de_comandos_ignorados() {
        assert_eq!(
            words("\\includegraphics[width=3cm]{img/foto} legenda \\usepackage[utf8]{inputenc}"),
            ["legenda"]
        );
    }

    #[test]
    fn acento_latex_marca_toque() {
        let t = scan("caf\\'e").tokens;
        assert!(t.iter().any(|t| t.touches_escape));
    }

    #[test]
    fn conta_math() {
        let s = scan("$a$ e \\(b\\) e \\[c\\] e $$d$$");
        assert_eq!((s.inline_math, s.display_math), (2, 2));
    }
}
