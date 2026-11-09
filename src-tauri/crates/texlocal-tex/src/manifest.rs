//! Manifesto `texlocal.toml` do projeto.

use std::fmt;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

pub const MANIFEST_FILE: &str = "texlocal.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    #[default]
    Pdflatex,
    Xelatex,
    Lualatex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Bibliography {
    Bibtex,
    Biber,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ProjectSection {
    pub main: String,
}

impl Default for ProjectSection {
    fn default() -> Self {
        Self {
            main: "main.tex".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct TexSection {
    pub engine: Engine,
    /// Ano mínimo do TeX Live.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texlive: Option<u16>,
    pub packages: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bibliography: Option<Bibliography>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CompileSection {
    pub auto: bool,
    pub shell_escape: bool,
    pub outdir: String,
}

impl Default for CompileSection {
    fn default() -> Self {
        Self {
            auto: true,
            shell_escape: false,
            outdir: ".texlocal/build".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Manifest {
    pub project: ProjectSection,
    pub tex: TexSection,
    pub compile: CompileSection,
    /// Configurações do editor por projeto (EP01/TL-105), preservadas como estão.
    #[serde(skip_serializing_if = "toml::Table::is_empty")]
    pub editor: toml::Table,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ManifestError {
    /// Erro de sintaxe/tipo no TOML, com posição (linha, coluna) quando conhecida.
    Parse {
        message: String,
        line: Option<usize>,
        column: Option<usize>,
    },
    Invalid {
        field: &'static str,
        reason: String,
    },
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse {
                message,
                line: Some(l),
                column: Some(c),
            } => write!(f, "texlocal.toml, linha {l}, coluna {c}: {message}"),
            Self::Parse { message, .. } => write!(f, "texlocal.toml: {message}"),
            Self::Invalid { field, reason } => write!(f, "texlocal.toml: {field}: {reason}"),
        }
    }
}

impl std::error::Error for ManifestError {}

fn invalid(field: &'static str, reason: &str) -> ManifestError {
    ManifestError::Invalid {
        field,
        reason: reason.into(),
    }
}

fn is_safe_relative(path: &str) -> bool {
    let p = Path::new(path);
    !path.is_empty()
        && p.is_relative()
        && !path.starts_with('/')
        && !path.starts_with('\\')
        && p.components().all(|c| matches!(c, Component::Normal(_)))
}

fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().map_or(0, |s| s.chars().count()) + 1;
    (line, column)
}

impl Manifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        if !is_safe_relative(&self.project.main) || !self.project.main.ends_with(".tex") {
            return Err(invalid(
                "project.main",
                "deve ser um caminho relativo dentro do projeto terminado em .tex",
            ));
        }
        if !is_safe_relative(&self.compile.outdir) {
            return Err(invalid(
                "compile.outdir",
                "deve ser um caminho relativo dentro do projeto",
            ));
        }
        if let Some(year) = self.tex.texlive {
            if !(2000..=2100).contains(&year) {
                return Err(invalid("tex.texlive", "ano fora do intervalo 2000-2100"));
            }
        }
        let valid_name = |p: &str| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'))
        };
        if let Some(bad) = self.tex.packages.iter().find(|p| !valid_name(p)) {
            return Err(ManifestError::Invalid {
                field: "tex.packages",
                reason: format!("nome de pacote inválido: {bad:?}"),
            });
        }
        Ok(())
    }

    pub fn from_toml(source: &str) -> Result<Self, ManifestError> {
        let manifest: Self = toml::from_str(source).map_err(|e| {
            let (line, column) = e.span().map_or((None, None), |s| {
                let (l, c) = line_col(source, s.start);
                (Some(l), Some(c))
            });
            ManifestError::Parse {
                message: e.message().to_string(),
                line,
                column,
            }
        })?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("Manifest é serializável")
    }
}

/// Pacotes do TeX Live inferidos a partir de `\usepackage`/`\documentclass` conhecidos.
const KNOWN_PACKAGES: [(&str, &str); 8] = [
    ("abntex2", "abntex2"),
    ("abntex2cite", "abntex2"),
    ("biblatex", "biblatex"),
    ("siunitx", "siunitx"),
    ("tikz", "pgf"),
    ("pgfplots", "pgfplots"),
    ("minted", "minted"),
    ("fontspec", "fontspec"),
];

/// Infere um manifesto a partir do conteúdo do arquivo principal e da existência de `.bib`.
pub fn infer(main: &str, main_source: &str, has_bib_file: bool) -> Manifest {
    let mut m = Manifest::default();
    m.project.main = main.to_string();

    let uses = |name: &str| {
        let re = regex::Regex::new(&format!(
            r"\\(?:usepackage|RequirePackage|documentclass)(?:\[[^\]]*\])?\{{[^}}]*\b{}\b[^}}]*\}}",
            regex::escape(name)
        ))
        .expect("regex válida");
        re.is_match(main_source)
    };

    if uses("fontspec") || uses("unicode-math") {
        m.tex.engine = Engine::Xelatex;
    }
    for (needle, package) in KNOWN_PACKAGES {
        if uses(needle) && !m.tex.packages.iter().any(|p| p == package) {
            m.tex.packages.push(package.to_string());
        }
    }
    if main_source.contains("\\addbibresource") || uses("biblatex") {
        m.tex.bibliography = Some(Bibliography::Biber);
        if !m.tex.packages.iter().any(|p| p == "biber") {
            m.tex.packages.push("biber".into());
        }
    } else if main_source.contains("\\bibliography{") || has_bib_file {
        m.tex.bibliography = Some(Bibliography::Bibtex);
    }
    m
}

/// Escolhe o arquivo principal de um conjunto de `.tex` da raiz do projeto.
pub fn pick_main(files: &[(String, String)]) -> Option<&str> {
    files
        .iter()
        .find(|(name, _)| name == "main.tex")
        .or_else(|| {
            files
                .iter()
                .find(|(_, src)| src.contains("\\documentclass"))
        })
        .map(|(name, _)| name.as_str())
}

/// Lê o manifesto de uma pasta de projeto; `Ok(None)` se não existir.
pub fn read(dir: &Path) -> Result<Option<Manifest>, String> {
    match std::fs::read_to_string(dir.join(MANIFEST_FILE)) {
        Ok(src) => Manifest::from_toml(&src)
            .map(Some)
            .map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Grava o manifesto (validado) com fim de linha LF.
pub fn write(dir: &Path, manifest: &Manifest) -> Result<(), String> {
    manifest.validate().map_err(|e| e.to_string())?;
    std::fs::write(dir.join(MANIFEST_FILE), manifest.to_toml()).map_err(|e| e.to_string())
}

/// Lê o manifesto ou o infere da pasta (e o cria) quando ainda não existe (TL-206).
pub fn read_or_create(dir: &Path) -> Result<Manifest, String> {
    if let Some(m) = read(dir)? {
        return Ok(m);
    }
    let mut files = Vec::new();
    let mut has_bib = false;
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".bib") {
            has_bib = true;
        } else if name.ends_with(".tex") && entry.path().is_file() {
            let src = std::fs::read_to_string(entry.path()).unwrap_or_default();
            files.push((name, src));
        }
    }
    files.sort();
    let manifest = match pick_main(&files) {
        Some(main) => {
            let src = &files.iter().find(|(n, _)| n == main).expect("existe").1;
            infer(main, src, has_bib)
        }
        None => Manifest::default(),
    };
    write(dir, &manifest)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[project]
main = "main.tex"
[tex]
engine = "xelatex"
texlive = 2026
packages = ["abntex2", "siunitx"]
bibliography = "biber"
[compile]
auto = true
shell_escape = false
outdir = ".texlocal/build"
[editor]
tab_size = 2
"#;

    #[test]
    fn parses_documented_schema_and_preserves_editor_section() {
        let m = Manifest::from_toml(SAMPLE).unwrap();
        assert_eq!(m.tex.engine, Engine::Xelatex);
        assert_eq!(m.tex.texlive, Some(2026));
        assert_eq!(m.tex.bibliography, Some(Bibliography::Biber));
        assert_eq!(m.editor["tab_size"].as_integer(), Some(2));
        let again = Manifest::from_toml(&m.to_toml()).unwrap();
        assert_eq!(again, m);
    }

    #[test]
    fn empty_file_uses_defaults() {
        assert_eq!(Manifest::from_toml("").unwrap(), Manifest::default());
    }

    #[test]
    fn syntax_error_reports_position() {
        let err = Manifest::from_toml("[tex]\nengine = \n").unwrap_err();
        assert!(matches!(err, ManifestError::Parse { line: Some(2), .. }));
        assert!(err.to_string().contains("linha 2"));
    }

    #[test]
    fn rejects_unknown_engine_and_keys() {
        assert!(Manifest::from_toml("[tex]\nengine = \"word\"").is_err());
        assert!(Manifest::from_toml("[tex]\nfoo = 1").is_err());
    }

    #[test]
    fn rejects_unsafe_paths_and_names() {
        for bad in [
            "[project]\nmain = \"../x.tex\"",
            "[project]\nmain = \"/etc/x.tex\"",
            "[project]\nmain = \"main.txt\"",
            "[compile]\noutdir = \"../out\"",
            "[tex]\npackages = [\"a b\"]",
            "[tex]\ntexlive = 1999",
        ] {
            assert!(
                matches!(Manifest::from_toml(bad), Err(ManifestError::Invalid { .. })),
                "deveria rejeitar: {bad}"
            );
        }
    }

    #[test]
    fn infers_from_sources() {
        let src = "\\documentclass[12pt]{abntex2}\n\\usepackage{siunitx}\n\\usepackage[backend=biber]{biblatex}\n\\addbibresource{r.bib}";
        let m = infer("main.tex", src, true);
        assert_eq!(m.tex.bibliography, Some(Bibliography::Biber));
        for p in ["abntex2", "siunitx", "biblatex", "biber"] {
            assert!(m.tex.packages.iter().any(|x| x == p), "faltou {p}");
        }
        assert_eq!(m.tex.engine, Engine::Pdflatex);

        let m = infer("main.tex", "\\usepackage{fontspec}", false);
        assert_eq!(m.tex.engine, Engine::Xelatex);
    }

    #[test]
    fn pick_main_prefers_main_tex() {
        let files = vec![
            ("a.tex".to_string(), "\\documentclass{article}".to_string()),
            ("main.tex".to_string(), String::new()),
        ];
        assert_eq!(pick_main(&files), Some("main.tex"));
        let files = vec![("a.tex".to_string(), "\\documentclass{article}".to_string())];
        assert_eq!(pick_main(&files), Some("a.tex"));
    }

    #[test]
    fn creates_manifest_on_first_open_and_reads_it_back() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("tese.tex"),
            "\\documentclass{abntex2}\n\\bibliography{refs}",
        )
        .unwrap();
        let created = read_or_create(tmp.path()).unwrap();
        assert_eq!(created.project.main, "tese.tex");
        assert_eq!(created.tex.bibliography, Some(Bibliography::Bibtex));
        let written = std::fs::read_to_string(tmp.path().join(MANIFEST_FILE)).unwrap();
        assert!(!written.contains('\r'));
        assert_eq!(read(tmp.path()).unwrap().unwrap(), created);
    }
}
