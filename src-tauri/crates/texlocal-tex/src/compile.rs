//! Serviço de compilação com `latexmk` (TL-401).
//!
//! Executa o latexmk na raiz do projeto, com o motor do manifesto, `-synctex=1`,
//! `-interaction=nonstopmode`, `-file-line-error` e saída em `-outdir`. O processo roda em grupo
//! próprio (cancelável, com timeout) e com o PATH apontando para a árvore TeX escolhida.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;

use crate::detect::find_executable;
use crate::diagnostics::{self, Category, Diagnostic, Severity};
use crate::logparse::{find_missing_files, MissingFile};
use crate::manifest::{Engine, Manifest};
use crate::process;

/// Linha do `.gitignore` que mantém o diretório de build fora do git.
pub const GITIGNORE_HEADER: &str = "# TexLocal: arquivos gerados pela compilação";

#[derive(Debug, Clone)]
pub struct CompileRequest {
    pub project_dir: PathBuf,
    /// Diretório `bin` da distribuição TeX escolhida. `None` usa o PATH do sistema.
    pub bin_dir: Option<PathBuf>,
    pub manifest: Manifest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CompileStatus {
    Success,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileOutcome {
    pub status: CompileStatus,
    /// PDF gerado (caminho absoluto), se existir.
    pub pdf: Option<String>,
    /// O PDF foi reescrito por esta compilação.
    pub pdf_updated: bool,
    /// Arquivo `.synctex.gz` (caminho absoluto), se existir.
    pub synctex: Option<String>,
    /// Conteúdo do `.log` do TeX.
    pub log: String,
    /// Saída do próprio latexmk (stdout + stderr).
    pub output: String,
    pub diagnostics: Vec<Diagnostic>,
    /// Arquivos que o TeX não encontrou (insumo da auto-instalação, TL-407).
    pub missing_files: Vec<MissingFile>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CompileErrorKind {
    LatexmkNotFound,
    MainNotFound,
    /// `texlocal.toml` ilegível ou inválido.
    Manifest,
    /// Já existe uma compilação deste projeto em andamento.
    Busy,
    Io,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileError {
    pub kind: CompileErrorKind,
    /// Chave de tradução da mensagem amigável.
    pub i18n_key: &'static str,
    pub details: String,
}

impl CompileError {
    pub fn new(kind: CompileErrorKind, details: impl Into<String>) -> Self {
        let i18n_key = match kind {
            CompileErrorKind::LatexmkNotFound => "compile.error.latexmkNotFound",
            CompileErrorKind::MainNotFound => "compile.error.mainNotFound",
            CompileErrorKind::Manifest => "compile.error.manifest",
            CompileErrorKind::Busy => "compile.error.busy",
            CompileErrorKind::Io => "compile.error.io",
        };
        Self {
            kind,
            i18n_key,
            details: details.into(),
        }
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl std::error::Error for CompileError {}

/// Trecho Perl (`latexmk -e`) que liga o `makeglossaries` ao ciclo do latexmk.
const GLOSSARIES_RULES: &str = "add_cus_dep('glo','gls',0,'texlocal_makeglossaries'); \
add_cus_dep('acn','acr',0,'texlocal_makeglossaries'); \
sub texlocal_makeglossaries { my ($base_name, $path) = fileparse($_[0]); pushd $path; \
my $return = system('makeglossaries', $base_name); popd; return $return; }";

/// Argumentos do latexmk para o manifesto. O BibTeX/Biber e o makeindex são acionados pelo
/// próprio latexmk conforme os arquivos auxiliares; `uses_glossaries` liga o `makeglossaries`.
pub fn latexmk_args(manifest: &Manifest, uses_glossaries: bool) -> Vec<String> {
    let mut args = vec![
        match manifest.tex.engine {
            Engine::Pdflatex => "-pdf",
            Engine::Xelatex => "-xelatex",
            Engine::Lualatex => "-lualatex",
        }
        .to_owned(),
        "-synctex=1".to_owned(),
        "-interaction=nonstopmode".to_owned(),
        "-file-line-error".to_owned(),
        if manifest.compile.shell_escape {
            "-shell-escape".to_owned()
        } else {
            "-no-shell-escape".to_owned()
        },
        format!("-outdir={}", manifest.compile.outdir),
    ];
    if uses_glossaries {
        args.push("-e".to_owned());
        args.push(GLOSSARIES_RULES.to_owned());
    }
    args.push(manifest.project.main.clone());
    args
}

/// PATH com `bin_dir` na frente, para que latexmk, biber e afins venham da árvore escolhida.
pub fn path_with(bin_dir: &Path, existing: Option<OsString>) -> OsString {
    let mut dirs = vec![bin_dir.to_path_buf()];
    if let Some(existing) = existing {
        dirs.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(dirs).unwrap_or_else(|_| bin_dir.as_os_str().to_owned())
}

/// Garante `/<outdir>/` no `.gitignore` do projeto (criando o arquivo se preciso).
pub fn ensure_gitignore(project_dir: &Path, outdir: &str) -> std::io::Result<()> {
    let path = project_dir.join(".gitignore");
    let entry = format!("/{}/", outdir.trim_matches('/').replace('\\', "/"));
    let current = fs::read_to_string(&path).unwrap_or_default();
    let covered = current.lines().any(|l| {
        let l = l.trim();
        l == entry || l == entry.trim_end_matches('/') || l == entry.trim_start_matches('/')
    });
    if covered {
        return Ok(());
    }
    let mut next = current;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    if !next.contains(GITIGNORE_HEADER) {
        next.push_str(GITIGNORE_HEADER);
        next.push('\n');
    }
    next.push_str(&entry);
    next.push('\n');
    fs::write(path, next)
}

/// `\include{pasta/arquivo}` grava `pasta/arquivo.aux` dentro do `-outdir`: o TeX não cria
/// diretórios, então espelhamos as pastas do projeto que têm `.tex`.
fn mirror_directories(project_dir: &Path, outdir: &Path) {
    fn walk(dir: &Path, rel: &Path, outdir: &Path, depth: u32) {
        if depth > 8 {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str == "node_modules" {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                let child_rel = rel.join(&name);
                let has_tex = fs::read_dir(entry.path())
                    .map(|d| {
                        d.flatten()
                            .any(|f| f.path().extension().is_some_and(|e| e == "tex"))
                    })
                    .unwrap_or(false);
                if has_tex {
                    let _ = fs::create_dir_all(outdir.join(&child_rel));
                }
                walk(&entry.path(), &child_rel, outdir, depth + 1);
            }
        }
    }
    walk(project_dir, Path::new(""), outdir, 0);
}

fn read_lossy(path: &Path) -> Option<String> {
    fs::read(path)
        .ok()
        .map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// Caminhos absolutos dentro do projeto viram relativos (com `/`).
fn relativize(diagnostics: &mut [Diagnostic], project_dir: &Path) {
    let root = project_dir.to_string_lossy().replace('\\', "/");
    let root = root.trim_end_matches('/');
    for d in diagnostics {
        if let Some(file) = d.file.as_mut() {
            if let Some(rest) = file.strip_prefix(root) {
                if let Some(rest) = rest.strip_prefix('/') {
                    *file = rest.to_owned();
                }
            }
        }
    }
}

fn tail(text: &str, max: usize) -> String {
    let trimmed = text.trim();
    let count = trimmed.chars().count();
    if count <= max {
        return trimmed.to_owned();
    }
    trimmed.chars().skip(count - max).collect()
}

/// Compila o projeto. Bloqueante: rode em uma thread própria. `cancel` interrompe o processo.
pub fn compile(req: &CompileRequest, cancel: &AtomicBool) -> Result<CompileOutcome, CompileError> {
    let manifest = &req.manifest;
    let main = &manifest.project.main;
    let main_path = req.project_dir.join(main);
    if !main_path.is_file() {
        return Err(CompileError::new(
            CompileErrorKind::MainNotFound,
            main_path.display().to_string(),
        ));
    }

    let latexmk = match &req.bin_dir {
        Some(dir) => find_executable(dir, "latexmk"),
        None => std::env::var_os("PATH").and_then(|path| {
            std::env::split_paths(&path).find_map(|dir| find_executable(&dir, "latexmk"))
        }),
    }
    .ok_or_else(|| {
        CompileError::new(
            CompileErrorKind::LatexmkNotFound,
            req.bin_dir
                .as_ref()
                .map_or_else(|| "PATH".to_owned(), |d| d.display().to_string()),
        )
    })?;

    let io = |e: std::io::Error| CompileError::new(CompileErrorKind::Io, e.to_string());
    let outdir = req.project_dir.join(&manifest.compile.outdir);
    fs::create_dir_all(&outdir).map_err(io)?;
    // Não falhar a compilação por causa do `.gitignore` (ex.: pasta somente leitura).
    let _ = ensure_gitignore(&req.project_dir, &manifest.compile.outdir);
    mirror_directories(&req.project_dir, &outdir);

    let uses_glossaries = fs::read_to_string(&main_path)
        .map(|s| s.contains("\\makeglossaries") || s.contains("{glossaries"))
        .unwrap_or(false);

    let stem = Path::new(main)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "main".to_owned());
    let pdf_path = outdir.join(format!("{stem}.pdf"));
    let synctex_path = outdir.join(format!("{stem}.synctex.gz"));

    let mut cmd = process::prepare(&latexmk);
    cmd.args(latexmk_args(manifest, uses_glossaries))
        .current_dir(&req.project_dir);
    if let Some(bin) = &req.bin_dir {
        cmd.env("PATH", path_with(bin, std::env::var_os("PATH")));
    }

    let started_at = SystemTime::now();
    let started = Instant::now();
    let timeout = Duration::from_secs(manifest.compile.timeout_secs);
    let stop = AtomicBool::new(false);
    let timed_out = AtomicBool::new(false);
    let finished = AtomicBool::new(false);

    let run = thread::scope(|scope| {
        scope.spawn(|| {
            while !finished.load(Ordering::SeqCst) {
                if cancel.load(Ordering::SeqCst) {
                    stop.store(true, Ordering::SeqCst);
                    return;
                }
                if started.elapsed() >= timeout {
                    timed_out.store(true, Ordering::SeqCst);
                    stop.store(true, Ordering::SeqCst);
                    return;
                }
                thread::sleep(Duration::from_millis(25));
            }
        });
        let result = process::run_streaming(cmd, &stop, &mut |_| {});
        finished.store(true, Ordering::SeqCst);
        result
    })
    .map_err(io)?;

    let status = if timed_out.load(Ordering::SeqCst) {
        CompileStatus::TimedOut
    } else if run.cancelled {
        CompileStatus::Cancelled
    } else if run.status.success() {
        CompileStatus::Success
    } else {
        CompileStatus::Failed
    };

    let log = read_lossy(&outdir.join(format!("{stem}.log"))).unwrap_or_default();
    let mut output = run.stdout;
    if !run.stderr.trim().is_empty() {
        output.push_str(&run.stderr);
    }

    let mut diagnostics = diagnostics::parse_latex_log(&log);
    if let Some(blg) = read_lossy(&outdir.join(format!("{stem}.blg"))) {
        diagnostics.extend(diagnostics::parse_bib_log(&blg));
    }
    relativize(&mut diagnostics, &req.project_dir);
    if status == CompileStatus::Failed && !diagnostics.iter().any(|d| d.severity == Severity::Error)
    {
        // O TeX nem chegou a escrever o log (ou o latexmk falhou antes): mostra a saída dele.
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            category: Category::Latex,
            file: None,
            line: None,
            message: tail(&output, 400),
            explanation: Some("latexmkFailed"),
            context: None,
        });
    }

    let modified = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok();
    let pdf_updated = modified(&pdf_path)
        // A granularidade do relógio de arquivos pode chegar a 2 s (FAT/ext3).
        .is_some_and(|m| m + Duration::from_secs(2) >= started_at);

    let missing_files = find_missing_files(&format!("{log}\n{output}"));

    Ok(CompileOutcome {
        status,
        pdf: pdf_path
            .is_file()
            .then(|| pdf_path.to_string_lossy().into_owned()),
        pdf_updated,
        synctex: synctex_path
            .is_file()
            .then(|| synctex_path.to_string_lossy().into_owned()),
        log,
        output,
        diagnostics,
        missing_files,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_follow_the_manifest() {
        let mut m = Manifest::default();
        m.project.main = "tese.tex".into();
        let args = latexmk_args(&m, false);
        assert_eq!(args.first().map(String::as_str), Some("-pdf"));
        for expected in [
            "-synctex=1",
            "-interaction=nonstopmode",
            "-file-line-error",
            "-no-shell-escape",
            "-outdir=.texlocal/build",
        ] {
            assert!(args.iter().any(|a| a == expected), "falta {expected}");
        }
        assert_eq!(args.last().map(String::as_str), Some("tese.tex"));
        assert!(!args.iter().any(|a| a == "-shell-escape"));

        m.tex.engine = Engine::Xelatex;
        assert_eq!(latexmk_args(&m, false)[0], "-xelatex");
        m.tex.engine = Engine::Lualatex;
        assert_eq!(latexmk_args(&m, false)[0], "-lualatex");
    }

    #[test]
    fn shell_escape_only_when_enabled() {
        let mut m = Manifest::default();
        m.compile.shell_escape = true;
        let args = latexmk_args(&m, false);
        assert!(args.iter().any(|a| a == "-shell-escape"));
        assert!(!args.iter().any(|a| a == "-no-shell-escape"));
    }

    #[test]
    fn glossaries_rules_are_opt_in() {
        let m = Manifest::default();
        assert!(!latexmk_args(&m, false).iter().any(|a| a == "-e"));
        let with = latexmk_args(&m, true);
        let at = with.iter().position(|a| a == "-e").expect("-e presente");
        assert!(with[at + 1].contains("makeglossaries"));
        assert_eq!(with.last().map(String::as_str), Some("main.tex"));
    }

    #[test]
    fn path_puts_the_chosen_tree_first() {
        let joined = path_with(Path::new("/opt/tl/bin"), Some(OsString::from("/usr/bin")));
        let dirs: Vec<_> = std::env::split_paths(&joined).collect();
        assert_eq!(dirs[0], PathBuf::from("/opt/tl/bin"));
        assert!(dirs.contains(&PathBuf::from("/usr/bin")));
    }

    #[test]
    fn gitignore_is_created_updated_and_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        ensure_gitignore(dir.path(), ".texlocal/build").unwrap();
        let first = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(first.contains("/.texlocal/build/"));
        ensure_gitignore(dir.path(), ".texlocal/build").unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join(".gitignore")).unwrap(),
            first
        );

        fs::write(dir.path().join(".gitignore"), "*.aux").unwrap();
        ensure_gitignore(dir.path(), ".texlocal/build").unwrap();
        let merged = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(merged.starts_with("*.aux\n"));
        assert!(merged.contains("/.texlocal/build/"));
    }

    #[test]
    fn gitignore_respects_an_existing_entry() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), ".texlocal/build/\n").unwrap();
        ensure_gitignore(dir.path(), ".texlocal/build").unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join(".gitignore")).unwrap(),
            ".texlocal/build/\n"
        );
    }

    #[test]
    fn missing_main_and_missing_latexmk_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let bin = tempfile::tempdir().unwrap();
        let req = CompileRequest {
            project_dir: dir.path().to_path_buf(),
            bin_dir: Some(bin.path().to_path_buf()),
            manifest: Manifest::default(),
        };
        let cancel = AtomicBool::new(false);
        assert_eq!(
            compile(&req, &cancel).unwrap_err().kind,
            CompileErrorKind::MainNotFound
        );
        fs::write(dir.path().join("main.tex"), "x").unwrap();
        let err = compile(&req, &cancel).unwrap_err();
        assert_eq!(err.kind, CompileErrorKind::LatexmkNotFound);
        assert_eq!(err.i18n_key, "compile.error.latexmkNotFound");
    }

    #[test]
    fn mirrors_directories_that_contain_tex() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("chapters/deep")).unwrap();
        fs::write(dir.path().join("chapters/a.tex"), "").unwrap();
        fs::create_dir_all(dir.path().join("images")).unwrap();
        fs::write(dir.path().join("images/a.png"), "").unwrap();
        let out = dir.path().join(".texlocal/build");
        fs::create_dir_all(&out).unwrap();
        mirror_directories(dir.path(), &out);
        assert!(out.join("chapters").is_dir());
        assert!(!out.join("images").exists());
        assert!(!out.join(".texlocal").exists());
    }

    #[cfg(unix)]
    mod with_fake_latexmk {
        use super::*;
        use std::os::unix::fs::PermissionsExt;
        use std::sync::Arc;

        /// Cria um `latexmk` falso: `body` é o corpo do script POSIX; `$OUT` é o outdir e `$MAIN`
        /// o arquivo principal.
        fn fake_latexmk(dir: &Path, body: &str) {
            let script = format!(
                "#!/bin/sh\nfor a in \"$@\"; do case \"$a\" in -outdir=*) OUT=\"${{a#-outdir=}}\";; esac; MAIN=\"$a\"; done\nSTEM=$(basename \"$MAIN\" .tex)\n{body}\n"
            );
            let path = dir.join("latexmk");
            fs::write(&path, script).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        fn project(main: &str) -> (tempfile::TempDir, tempfile::TempDir, CompileRequest) {
            let proj = tempfile::tempdir().unwrap();
            let bin = tempfile::tempdir().unwrap();
            fs::write(proj.path().join(main), "\\documentclass{article}").unwrap();
            let mut manifest = Manifest::default();
            manifest.project.main = main.into();
            let req = CompileRequest {
                project_dir: proj.path().to_path_buf(),
                bin_dir: Some(bin.path().to_path_buf()),
                manifest,
            };
            (proj, bin, req)
        }

        #[test]
        fn successful_run_reports_pdf_synctex_and_runs_in_the_project() {
            let (proj, bin, req) = project("main.tex");
            fake_latexmk(
                bin.path(),
                "mkdir -p \"$OUT\"\n\
                 echo \"args: $*\" > \"$OUT/args.txt\"\n\
                 echo \"PATH=$PATH\" > \"$OUT/env.txt\"\n\
                 pwd > \"$OUT/pwd.txt\"\n\
                 printf '(./main.tex\\nOutput written on main.pdf.\\n)' > \"$OUT/$STEM.log\"\n\
                 echo pdf > \"$OUT/$STEM.pdf\"\n\
                 echo sync > \"$OUT/$STEM.synctex.gz\"\n\
                 echo 'Latexmk: All targets are up-to-date'",
            );
            let outcome = compile(&req, &AtomicBool::new(false)).unwrap();
            assert_eq!(outcome.status, CompileStatus::Success);
            assert!(outcome.pdf_updated);
            assert!(outcome.pdf.as_deref().unwrap().ends_with("main.pdf"));
            assert!(outcome
                .synctex
                .as_deref()
                .unwrap()
                .ends_with("main.synctex.gz"));
            assert!(outcome.output.contains("All targets"));
            assert!(outcome.diagnostics.is_empty());

            let out = proj.path().join(".texlocal/build");
            let args = fs::read_to_string(out.join("args.txt")).unwrap();
            assert!(args.contains("-pdf") && args.contains("-synctex=1"));
            assert!(args.contains("-outdir=.texlocal/build"));
            let env = fs::read_to_string(out.join("env.txt")).unwrap();
            assert!(env.starts_with(&format!("PATH={}", bin.path().display())));
            let pwd = fs::read_to_string(out.join("pwd.txt")).unwrap();
            assert_eq!(
                Path::new(pwd.trim()).canonicalize().unwrap(),
                proj.path().canonicalize().unwrap()
            );
            let ignore = fs::read_to_string(proj.path().join(".gitignore")).unwrap();
            assert!(ignore.contains("/.texlocal/build/"));
        }

        #[test]
        fn failed_run_surfaces_parsed_errors() {
            let (_proj, bin, req) = project("main.tex");
            fake_latexmk(
                bin.path(),
                "mkdir -p \"$OUT\"\n\
                 printf '(./main.tex\\n./main.tex:3: Undefined control sequence.\\nl.3 \\\\foo\\n\\n)' > \"$OUT/$STEM.log\"\n\
                 exit 12",
            );
            let outcome = compile(&req, &AtomicBool::new(false)).unwrap();
            assert_eq!(outcome.status, CompileStatus::Failed);
            assert!(!outcome.pdf_updated);
            assert_eq!(outcome.diagnostics.len(), 1);
            assert_eq!(outcome.diagnostics[0].line, Some(3));
            assert_eq!(outcome.diagnostics[0].file.as_deref(), Some("main.tex"));
        }

        #[test]
        fn failure_without_a_log_reports_latexmk_output() {
            let (_proj, bin, req) = project("main.tex");
            fake_latexmk(
                bin.path(),
                "echo 'Latexmk: Could not find rc file' >&2\nexit 11",
            );
            let outcome = compile(&req, &AtomicBool::new(false)).unwrap();
            assert_eq!(outcome.status, CompileStatus::Failed);
            let d = &outcome.diagnostics[0];
            assert_eq!(d.severity, Severity::Error);
            assert_eq!(d.explanation, Some("latexmkFailed"));
            assert!(d.message.contains("Could not find rc file"));
        }

        #[test]
        fn main_in_a_subdirectory_uses_its_own_stem() {
            let (proj, bin, mut req) = project("main.tex");
            fs::create_dir_all(proj.path().join("docs")).unwrap();
            fs::write(proj.path().join("docs/artigo.tex"), "x").unwrap();
            req.manifest.project.main = "docs/artigo.tex".into();
            fake_latexmk(
                bin.path(),
                "mkdir -p \"$OUT\"\necho pdf > \"$OUT/$STEM.pdf\"\n: > \"$OUT/$STEM.log\"",
            );
            let outcome = compile(&req, &AtomicBool::new(false)).unwrap();
            assert!(outcome.pdf.as_deref().unwrap().ends_with("artigo.pdf"));
        }

        #[test]
        fn cancel_kills_the_process_tree() {
            let (_proj, bin, req) = project("main.tex");
            fake_latexmk(bin.path(), "sleep 30");
            let cancel = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&cancel);
            let canceller = thread::spawn(move || {
                thread::sleep(Duration::from_millis(300));
                flag.store(true, Ordering::SeqCst);
            });
            let started = Instant::now();
            let outcome = compile(&req, &cancel).unwrap();
            canceller.join().unwrap();
            assert_eq!(outcome.status, CompileStatus::Cancelled);
            assert!(started.elapsed() < Duration::from_secs(10));
        }

        #[test]
        fn timeout_stops_a_slow_compilation() {
            let (_proj, bin, mut req) = project("main.tex");
            req.manifest.compile.timeout_secs = 1;
            fake_latexmk(bin.path(), "sleep 30");
            let started = Instant::now();
            let outcome = compile(&req, &AtomicBool::new(false)).unwrap();
            assert_eq!(outcome.status, CompileStatus::TimedOut);
            assert!(started.elapsed() < Duration::from_secs(10));
        }

        #[test]
        fn missing_packages_are_collected_for_auto_install() {
            let (_proj, bin, req) = project("main.tex");
            fake_latexmk(
                bin.path(),
                "mkdir -p \"$OUT\"\n\
                 printf \"./main.tex:2: LaTeX Error: File \\`siunitx.sty' not found.\\n\" > \"$OUT/$STEM.log\"\n\
                 exit 12",
            );
            let outcome = compile(&req, &AtomicBool::new(false)).unwrap();
            assert_eq!(outcome.missing_files.len(), 1);
            assert_eq!(outcome.missing_files[0].name, "siunitx.sty");
        }
    }
}
