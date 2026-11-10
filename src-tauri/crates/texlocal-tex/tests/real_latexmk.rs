//! Teste de integração com o latexmk/TeX reais. É ignorado (passa sem fazer nada) quando o
//! `latexmk` não está no PATH, para não quebrar ambientes sem TeX.

use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use texlocal_tex::compile::{compile, CompileRequest, CompileStatus};
use texlocal_tex::diagnostics::Severity;
use texlocal_tex::manifest::Manifest;
use texlocal_tex::synctex::SyncIndex;

fn has_latexmk() -> bool {
    std::env::var_os("PATH").is_some_and(|p| {
        std::env::split_paths(&p)
            .any(|d| texlocal_tex::detect::find_executable(&d, "latexmk").is_some())
    })
}

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

const MAIN: &str = "\\documentclass{article}\n\\begin{document}\nHello world, see \\cite{knuth84}.\n\n\\include{chapters/intro}\n\n\\bibliographystyle{plain}\n\\bibliography{refs}\n\\end{document}\n";
const INTRO: &str = "\\section{Intro}\nThis is the first paragraph of the introduction.\n\nSecond paragraph here with some more words to fill the line.\n";
const BIB: &str = "@book{knuth84,\n  author = {Donald Knuth},\n  title = {The TeXbook},\n  publisher = {Addison-Wesley},\n  year = {1984}\n}\n";

fn request(dir: &Path) -> CompileRequest {
    CompileRequest {
        project_dir: dir.to_path_buf(),
        bin_dir: None,
        manifest: Manifest::default(),
        force: false,
    }
}

#[test]
fn compiles_reports_errors_and_syncs_with_real_tex() {
    if !has_latexmk() {
        eprintln!("latexmk ausente: teste ignorado");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "main.tex", MAIN);
    write(root, "chapters/intro.tex", INTRO);
    write(root, "refs.bib", BIB);
    let cancel = AtomicBool::new(false);

    // 1) Documento válido: \include em subpasta, BibTeX, PDF e SyncTeX.
    let ok = compile(&request(root), &cancel).unwrap();
    assert_eq!(
        ok.status,
        CompileStatus::Success,
        "{}\n{}",
        ok.output,
        ok.log
    );
    assert!(ok.pdf_updated);
    assert!(
        ok.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "{:#?}",
        ok.diagnostics
    );
    assert!(
        root.join(".texlocal/build/main.bbl").exists(),
        "BibTeX deveria ter rodado"
    );
    assert!(fs::read_to_string(root.join(".gitignore"))
        .unwrap()
        .contains("/.texlocal/build/"));

    // 2) SyncTeX: direto e inverso, no arquivo incluído.
    let sync = SyncIndex::load(Path::new(ok.synctex.as_deref().expect("synctex"))).unwrap();
    let intro = root.join("chapters/intro.tex");
    let fwd = sync.forward(&intro, 2, root).expect("busca direta");
    // \include força uma nova página para o capítulo.
    assert_eq!(fwd.page, 2);
    // Confere a geometria com o binário oficial `synctex`, quando disponível.
    if let Ok(out) = std::process::Command::new("synctex")
        .args(["view", "-i", &format!("2:1:{}", intro.display()), "-o"])
        .arg(ok.pdf.as_deref().unwrap())
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let field = |name: &str| -> Option<f64> {
            text.lines()
                .find_map(|l| l.strip_prefix(&format!("{name}:")))
                .and_then(|v| v.trim().parse().ok())
        };
        if let (Some(h), Some(v), Some(w), Some(height)) =
            (field("h"), field("v"), field("W"), field("H"))
        {
            let r = fwd.rects[0];
            let close = |a: f64, b: f64| (a - b).abs() < 0.05;
            assert!(close(r.x, h), "x: {} != {h}", r.x);
            assert!(close(r.w, w), "w: {} != {w}", r.w);
            assert!(close(r.h, height), "h: {} != {height}", r.h);
            assert!(close(r.y + r.h, v), "baseline: {} != {v}", r.y + r.h);
        }
    }
    let rect = fwd.rects[0];
    let back = sync
        .inverse(fwd.page, rect.x + rect.w / 3.0, rect.y + rect.h / 2.0, root)
        .expect("busca inversa");
    assert!(back.file.ends_with("chapters/intro.tex"), "{back:?}");
    assert!((1..=4).contains(&back.line), "{back:?}");
    let main_fwd = sync
        .forward(&root.join("main.tex"), 3, root)
        .expect("busca direta no principal");
    let r = main_fwd.rects[0];
    let back = sync
        .inverse(main_fwd.page, r.x + 5.0, r.y + r.h / 2.0, root)
        .unwrap();
    assert!(
        back.file.ends_with("main.tex") && back.line == 3,
        "{back:?}"
    );

    // 3) Erros: comando indefinido no principal e ambiente inexistente no capítulo.
    write(
        root,
        "main.tex",
        &MAIN.replace("Hello world", "\\foo Hello world"),
    );
    write(
        root,
        "chapters/intro.tex",
        &format!("{INTRO}\\begin{{nope}}x\\end{{nope}}\n"),
    );
    let bad = compile(&request(root), &cancel).unwrap();
    assert_eq!(bad.status, CompileStatus::Failed);
    let errors: Vec<_> = bad
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.iter().any(|d| d.file.as_deref() == Some("main.tex")
            && d.line == Some(3)
            && d.message.contains("Undefined control sequence")),
        "{errors:#?}"
    );
    assert!(
        errors
            .iter()
            .any(|d| d.file.as_deref() == Some("chapters/intro.tex")
                && d.message.contains("Environment nope undefined")),
        "{errors:#?}"
    );
}

#[test]
fn missing_package_is_reported_for_auto_install() {
    if !has_latexmk() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "main.tex", "\\documentclass{article}\n\\usepackage{definitelynotapackage}\n\\begin{document}x\\end{document}\n");
    let out = compile(&request(dir.path()), &AtomicBool::new(false)).unwrap();
    assert_eq!(out.status, CompileStatus::Failed);
    assert!(
        out.missing_files
            .iter()
            .any(|f| f.name == "definitelynotapackage.sty"),
        "{:#?}",
        out.missing_files
    );
}
