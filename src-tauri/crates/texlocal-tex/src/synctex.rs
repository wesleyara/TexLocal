//! SyncTeX em Rust puro (TL-406): lê o `.synctex.gz` e converte posição do PDF <-> fonte,
//! sem depender do binário `synctex`.
//!
//! Formato (versão 1): um preâmbulo (`Input:N:caminho`, `Magnification:`, `Unit:`, ...) e, depois
//! de `Content:`, registros por página: `{N`/`}N` (folha), `[`/`]` (vbox), `(`/`)` (hbox) e os
//! folhas `h`, `v`, `k` (kern), `g` (cola), `$` (matemática) e `x`. Cada registro tem a forma
//! `<tag>,<linha>:<x>,<y>[:<w>[,<h>[,<d>]]]` em sp (escalados por `Unit` e `Magnification`).
//!
//! As coordenadas devolvidas estão em pontos PDF (big points), com origem no canto superior
//! esquerdo da página; `y` é a linha de base e a caixa ocupa `y - h` até `y + d`.

use std::io::{self, BufRead, BufReader, Read};
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use serde::Serialize;

/// 1 sp em pontos PDF: `65536 * 72.27 / 72`.
const SP_PER_BP: f64 = 65781.76;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Hbox com filhos (`(`).
    HBox,
    /// Hbox vazia (`h`).
    VoidHBox,
    /// Cola, kern, matemática (`g`, `k`, `$`, `x`).
    Leaf,
}

#[derive(Debug, Clone)]
struct Node {
    kind: Kind,
    page: u32,
    input: u32,
    line: u32,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    d: f64,
    /// Hbox que contém este nó.
    parent: Option<usize>,
}

impl Node {
    fn top(&self) -> f64 {
        self.y - self.h
    }
    fn bottom(&self) -> f64 {
        self.y + self.d
    }
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.top() && y <= self.bottom()
    }
    fn area(&self) -> f64 {
        self.w.max(0.0) * (self.h + self.d).max(0.0)
    }
    /// Distância (em bp) do ponto ao retângulo da caixa.
    fn distance(&self, x: f64, y: f64) -> f64 {
        let dx = (self.x - x).max(0.0).max(x - (self.x + self.w));
        let dy = (self.top() - y).max(0.0).max(y - self.bottom());
        dx.hypot(dy)
    }
}

/// Retângulo no PDF (origem no canto superior esquerdo, em bp).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Resultado da busca direta (fonte -> PDF).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardResult {
    /// Página (1-based).
    pub page: u32,
    pub rects: Vec<SyncRect>,
}

/// Resultado da busca inversa (PDF -> fonte).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePosition {
    /// Caminho absoluto do arquivo.
    pub file: String,
    /// Linha (1-based).
    pub line: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SyncIndex {
    inputs: Vec<(u32, String)>,
    nodes: Vec<Node>,
}

#[derive(Debug)]
pub enum SyncError {
    Io(io::Error),
    /// Não parece um arquivo SyncTeX.
    Format(String),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Format(m) => write!(f, "synctex inválido: {m}"),
        }
    }
}

impl std::error::Error for SyncError {}

impl From<io::Error> for SyncError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// Normaliza lexicamente: `/` como separador, resolve `.` e `..`; relativos são juntados a `base`.
fn normalize(path: &str, base: &Path) -> String {
    let unified = path.replace('\\', "/");
    let p = Path::new(&unified);
    let joined = if p.is_absolute() || unified.chars().nth(1) == Some(':') {
        PathBuf::from(&unified)
    } else {
        base.join(p)
    };
    let mut out = PathBuf::new();
    for c in joined.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out.to_string_lossy().replace('\\', "/")
}

/// Chave de comparação: o Windows não diferencia maiúsculas em caminhos.
fn path_key(path: &str, base: &Path) -> String {
    let s = normalize(path, base);
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s
    }
}

fn parse_numbers(text: &str) -> Vec<i64> {
    text.split(',')
        .filter_map(|p| p.trim().parse::<i64>().ok())
        .collect()
}

impl SyncIndex {
    /// Lê um `.synctex.gz` (ou um `.synctex` sem compressão).
    pub fn load(path: &Path) -> Result<Self, SyncError> {
        let file = std::fs::File::open(path)?;
        let mut reader = BufReader::new(file);
        let magic = reader.fill_buf()?;
        if magic.starts_with(&[0x1f, 0x8b]) {
            Self::parse(BufReader::new(GzDecoder::new(reader)))
        } else {
            Self::parse(reader)
        }
    }

    pub fn parse<R: BufRead>(reader: R) -> Result<Self, SyncError> {
        let mut index = Self::default();
        let mut unit = 1.0_f64;
        let mut magnification = 1000.0_f64;
        let mut x_offset = 0.0_f64;
        let mut y_offset = 0.0_f64;
        let mut in_content = false;
        let mut saw_header = false;

        let mut page = 0_u32;
        // Pilha de caixas abertas: Some(índice) para hbox, None para vbox.
        let mut stack: Vec<Option<usize>> = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if !in_content {
                if line.starts_with("SyncTeX Version") {
                    saw_header = true;
                } else if let Some(rest) = line.strip_prefix("Input:") {
                    if let Some((id, path)) = rest.split_once(':') {
                        if let Ok(id) = id.parse() {
                            index.inputs.push((id, path.to_owned()));
                        }
                    }
                } else if let Some(v) = line.strip_prefix("Magnification:") {
                    magnification = v.trim().parse().unwrap_or(1000.0);
                } else if let Some(v) = line.strip_prefix("Unit:") {
                    unit = v.trim().parse().unwrap_or(1.0);
                } else if let Some(v) = line.strip_prefix("X Offset:") {
                    x_offset = v.trim().parse().unwrap_or(0.0);
                } else if let Some(v) = line.strip_prefix("Y Offset:") {
                    y_offset = v.trim().parse().unwrap_or(0.0);
                } else if line.starts_with("Content:") {
                    if !saw_header {
                        return Err(SyncError::Format("cabeçalho ausente".into()));
                    }
                    in_content = true;
                }
                continue;
            }

            // Arquivos incluídos depois da primeira página são declarados no meio do conteúdo.
            if let Some(rest) = line.strip_prefix("Input:") {
                if let Some((id, path)) = rest.split_once(':') {
                    if let Ok(id) = id.parse() {
                        index.inputs.push((id, path.to_owned()));
                    }
                }
                continue;
            }

            let scale = unit * magnification / 1000.0 / SP_PER_BP;
            let (x_off, y_off) = (x_offset * scale, y_offset * scale);
            let mut chars = line.chars();
            let Some(tag) = chars.next() else { continue };
            let rest = chars.as_str();
            match tag {
                '{' => page = rest.trim().parse().unwrap_or(page + 1),
                '}' => {}
                '[' => stack.push(None),
                ']' => {
                    stack.pop();
                }
                ')' => {
                    stack.pop();
                }
                '(' | 'h' | 'v' | 'k' | 'g' | '$' | 'x' => {
                    // `v` (vbox vazia) não localiza texto.
                    let Some(rec) = Record::parse(rest) else {
                        if tag == '(' {
                            stack.push(None);
                        }
                        continue;
                    };
                    let parent = stack.iter().rev().find_map(|e| *e);
                    let to_bp = |v: i64| v as f64 * scale;
                    let (kind, w, h, d) = match tag {
                        '(' => (Kind::HBox, rec.w, rec.h, rec.d),
                        'h' => (Kind::VoidHBox, rec.w, rec.h, rec.d),
                        'k' => (Kind::Leaf, rec.w, 0, 0),
                        _ => (Kind::Leaf, 0, 0, 0),
                    };
                    if tag == 'v' {
                        continue;
                    }
                    let mut node = Node {
                        kind,
                        page,
                        input: rec.input,
                        line: rec.line,
                        x: to_bp(rec.x) + x_off,
                        y: to_bp(rec.y) + y_off,
                        w: to_bp(w),
                        h: to_bp(h),
                        d: to_bp(d),
                        parent,
                    };
                    if node.kind == Kind::Leaf {
                        if let Some(p) = parent {
                            node.h = index.nodes[p].h;
                            node.d = index.nodes[p].d;
                        }
                    }
                    index.nodes.push(node);
                    if tag == '(' {
                        stack.push(Some(index.nodes.len() - 1));
                    }
                }
                'P' if line.starts_with("Postamble:") => break,
                _ => {}
            }
        }

        if !saw_header {
            return Err(SyncError::Format("não é um arquivo SyncTeX".into()));
        }
        Ok(index)
    }

    /// Ids de `Input:` que correspondem a `file`.
    fn input_ids(&self, file: &Path, base: &Path) -> Vec<u32> {
        let wanted = path_key(&file.to_string_lossy(), base);
        self.inputs
            .iter()
            .filter(|(_, p)| path_key(p, base) == wanted)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Busca direta: onde, no PDF, está a linha `line` de `file`.
    pub fn forward(&self, file: &Path, line: u32, base: &Path) -> Option<ForwardResult> {
        let ids = self.input_ids(file, base);
        if ids.is_empty() {
            return None;
        }
        let in_file = |n: &Node| ids.contains(&n.input);
        // Linha exata; senão a mais próxima (empate: a anterior).
        let best_line = self
            .nodes
            .iter()
            .filter(|n| in_file(n))
            .map(|n| n.line)
            .min_by_key(|l| (l.abs_diff(line), *l))?;
        let on_line = || {
            self.nodes
                .iter()
                .filter(|n| in_file(n) && n.line == best_line)
        };
        // A hbox de um parágrafo carrega a linha do início dele em todas as linhas visuais; as
        // folhas (cola, kern, matemática) dizem onde a linha de fato está. Só usa hboxes sem elas.
        let leaves: Vec<&Node> = on_line().filter(|n| n.kind != Kind::HBox).collect();
        let matches: Vec<&Node> = if leaves.is_empty() {
            on_line().collect()
        } else {
            leaves
        };
        let page = matches.first()?.page;

        let mut rects: Vec<SyncRect> = Vec::new();
        for n in matches.iter().filter(|n| n.page == page) {
            // A linha inteira (hbox pai) é mais legível que o ponto exato da cola/kern.
            let bx = match (n.kind, n.parent) {
                (Kind::Leaf, Some(p)) => &self.nodes[p],
                _ => n,
            };
            let rect = SyncRect {
                x: bx.x,
                y: bx.top(),
                w: bx.w,
                h: bx.h + bx.d,
            };
            if !rects.contains(&rect) {
                rects.push(rect);
            }
        }
        rects.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
        rects.truncate(24);
        Some(ForwardResult { page, rects })
    }

    /// Busca inversa: arquivo e linha para um ponto `(x, y)` (bp, canto superior esquerdo) de
    /// `page` (1-based).
    pub fn inverse(&self, page: u32, x: f64, y: f64, base: &Path) -> Option<SourcePosition> {
        let boxes = || {
            self.nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.page == page && n.kind == Kind::HBox)
        };
        // 1) hbox mais interna (menor área) que contém o ponto; 2) senão, a mais próxima.
        let container = boxes()
            .filter(|(_, n)| n.contains(x, y))
            .min_by(|(ia, a), (ib, b)| a.area().total_cmp(&b.area()).then(ib.cmp(ia)))
            .or_else(|| {
                boxes().min_by(|(_, a), (_, b)| a.distance(x, y).total_cmp(&b.distance(x, y)))
            })
            .map(|(i, _)| i);
        let void_box = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.page == page && n.kind == Kind::VoidHBox && n.contains(x, y))
            .min_by(|(_, a), (_, b)| a.area().total_cmp(&b.area()))
            .map(|(i, _)| i);

        let chosen = match (void_box, container) {
            (Some(v), _) => v,
            (None, Some(c)) => {
                // Entre os filhos da hbox, o mais próximo horizontalmente.
                self.nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| n.parent == Some(c) && n.line > 0)
                    .min_by(|(_, a), (_, b)| a.distance(x, y).total_cmp(&b.distance(x, y)))
                    .map_or(c, |(i, _)| i)
            }
            (None, None) => return None,
        };
        let node = &self.nodes[chosen];
        let path = self
            .inputs
            .iter()
            .find(|(id, _)| *id == node.input)
            .map(|(_, p)| p)?;
        Some(SourcePosition {
            file: normalize(path, base),
            line: node.line,
        })
    }

    pub fn pages(&self) -> u32 {
        self.nodes.iter().map(|n| n.page).max().unwrap_or(0)
    }
}

struct Record {
    input: u32,
    line: u32,
    x: i64,
    y: i64,
    w: i64,
    h: i64,
    d: i64,
}

impl Record {
    /// `<tag>,<linha>[,<coluna>]:<x>,<y>[:<w>[,<h>[,<d>]]]`
    fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split(':');
        let ids = parse_numbers(parts.next()?);
        let pos = parse_numbers(parts.next()?);
        let size = parts.next().map(parse_numbers).unwrap_or_default();
        let (&input, &line) = (ids.first()?, ids.get(1)?);
        Some(Self {
            input: u32::try_from(input).ok()?,
            line: u32::try_from(line).ok()?,
            x: *pos.first()?,
            y: *pos.get(1)?,
            w: size.first().copied().unwrap_or(0),
            h: size.get(1).copied().unwrap_or(0),
            d: size.get(2).copied().unwrap_or(0),
        })
    }
}

/// Lê todo o conteúdo comprimido (utilitário para testes e depuração).
pub fn read_gz(path: &Path) -> io::Result<String> {
    let mut out = String::new();
    GzDecoder::new(std::fs::File::open(path)?).read_to_string(&mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    /// 1 bp em sp, para escrever coordenadas legíveis nos testes.
    const BP: i64 = 65782; // ~65781.76

    fn sample() -> String {
        // Página 1 (612x792 bp). Parágrafo com duas linhas:
        //   linha de texto 1 (fonte linha 3): baseline y=100bp, altura 10, profundidade 3
        //   linha de texto 2 (fonte linha 4): baseline y=114bp
        // Em outro arquivo (capítulo), uma linha na página 2.
        let b = |v: i64| v * BP;
        format!(
            "SyncTeX Version:1\n\
             Input:1:./main.tex\n\
             Input:2:/proj/chapters/intro.tex\n\
             Output:pdf\n\
             Magnification:1000\n\
             Unit:1\n\
             X Offset:0\n\
             Y Offset:0\n\
             Content:\n\
             !100\n\
             {{1\n\
             [1,1:{x0},{y0}:{w},{h},0\n\
             (1,3:{x0},{y1}:{w},{h1},{d1}\n\
             g1,3:{x0},{y1}\n\
             k1,3:{xk},{y1}:{wk}\n\
             g1,3:{xg},{y1}\n\
             )\n\
             (1,3:{x0},{y2}:{w},{h1},{d1}\n\
             g1,4:{x0},{y2}\n\
             g1,4:{xg},{y2}\n\
             )\n\
             ]\n\
             }}1\n\
             {{2\n\
             [2,1:{x0},{y0}:{w},{h},0\n\
             (2,10:{x0},{y1}:{w},{h1},{d1}\n\
             x2,10:{x0},{y1}\n\
             )\n\
             ]\n\
             }}2\n\
             Postamble:\n\
             Count:9\n",
            x0 = b(72),
            xk = b(200),
            xg = b(400),
            wk = b(10),
            y0 = b(72),
            y1 = b(100),
            y2 = b(114),
            w = b(468),
            h = b(648),
            h1 = b(10),
            d1 = b(3),
        )
    }

    fn index() -> SyncIndex {
        SyncIndex::parse(sample().as_bytes()).unwrap()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.05
    }

    #[test]
    fn parses_inputs_pages_and_nodes() {
        let idx = index();
        assert_eq!(idx.pages(), 2);
        assert_eq!(idx.inputs.len(), 2);
        assert!(!idx.nodes.is_empty());
    }

    #[test]
    fn forward_finds_the_line_rectangle() {
        let idx = index();
        let base = Path::new("/proj");
        let r = idx.forward(Path::new("/proj/main.tex"), 4, base).unwrap();
        assert_eq!(r.page, 1);
        assert_eq!(r.rects.len(), 1);
        let rect = r.rects[0];
        // A linha 3 só aparece na primeira linha visual, apesar de o parágrafo inteiro herdá-la.
        let first = idx.forward(Path::new("/proj/main.tex"), 3, base).unwrap();
        assert_eq!(first.rects.len(), 1);
        assert!(close(first.rects[0].y, 90.0), "{:?}", first.rects[0]);
        // y = baseline 114 - altura 10; altura total 13.
        assert!(close(rect.y, 104.0), "{rect:?}");
        assert!(close(rect.h, 13.0), "{rect:?}");
        assert!(close(rect.x, 72.0) && close(rect.w, 468.0), "{rect:?}");
    }

    #[test]
    fn forward_works_with_included_files_and_page_two() {
        let idx = index();
        let r = idx
            .forward(
                Path::new("/proj/chapters/intro.tex"),
                10,
                Path::new("/proj"),
            )
            .unwrap();
        assert_eq!(r.page, 2);
    }

    #[test]
    fn forward_uses_the_closest_line_when_absent() {
        let idx = index();
        let r = idx
            .forward(Path::new("/proj/main.tex"), 9, Path::new("/proj"))
            .unwrap();
        // A linha mais próxima de 9 em main.tex é a 4 (última).
        assert!(close(r.rects[0].y, 104.0));
    }

    #[test]
    fn forward_unknown_file_is_none() {
        let idx = index();
        assert!(idx
            .forward(Path::new("/proj/other.tex"), 1, Path::new("/proj"))
            .is_none());
    }

    #[test]
    fn inverse_picks_the_nearest_child_of_the_innermost_box() {
        let idx = index();
        let base = Path::new("/proj");
        // Primeira linha (baseline 100): clique perto da cola em x=400 -> linha 3.
        let p = idx.inverse(1, 405.0, 98.0, base).unwrap();
        assert_eq!(p.file, "/proj/main.tex");
        assert_eq!(p.line, 3);
        // Segunda linha (baseline 114): linha 4, mesmo que o pai seja "linha 3".
        let p = idx.inverse(1, 80.0, 112.0, base).unwrap();
        assert_eq!(p.line, 4);
    }

    #[test]
    fn inverse_resolves_included_file_paths() {
        let idx = index();
        let p = idx.inverse(2, 100.0, 99.0, Path::new("/proj")).unwrap();
        assert_eq!(p.file, "/proj/chapters/intro.tex");
        assert_eq!(p.line, 10);
    }

    #[test]
    fn inverse_outside_any_box_falls_back_to_the_nearest_line() {
        let idx = index();
        let p = idx.inverse(1, 30.0, 400.0, Path::new("/proj")).unwrap();
        assert_eq!(p.file, "/proj/main.tex");
        assert!(p.line == 3 || p.line == 4);
        assert!(idx.inverse(9, 1.0, 1.0, Path::new("/proj")).is_none());
    }

    #[test]
    fn inputs_declared_in_the_middle_of_the_content_are_registered() {
        // O pdfTeX só declara um arquivo incluído quando ele aparece pela primeira vez.
        let text = sample().replace(
            "{2\n",
            "Input:3:/proj/late.tex\n{2\n(3,7:0,0:1000,1000,0\n)\n",
        );
        let idx = SyncIndex::parse(text.as_bytes()).unwrap();
        let r = idx
            .forward(Path::new("/proj/late.tex"), 7, Path::new("/proj"))
            .expect("arquivo declarado depois de Content:");
        assert_eq!(r.page, 2);
    }

    #[test]
    fn rejects_garbage_and_reads_gzip_files() {
        assert!(SyncIndex::parse("hello\nworld\n".as_bytes()).is_err());

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("main.synctex.gz");
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(sample().as_bytes()).unwrap();
        std::fs::write(&path, enc.finish().unwrap()).unwrap();
        let idx = SyncIndex::load(&path).unwrap();
        assert_eq!(idx.pages(), 2);

        let plain = dir.path().join("plain.synctex");
        std::fs::write(&plain, sample()).unwrap();
        assert_eq!(SyncIndex::load(&plain).unwrap().pages(), 2);
        assert_eq!(read_gz(&path).unwrap(), sample());
    }

    #[test]
    fn normalization_handles_dot_segments_and_backslashes() {
        let base = Path::new("/proj");
        assert_eq!(normalize("./a/../b/c.tex", base), "/proj/b/c.tex");
        assert_eq!(normalize("/x/./y.tex", base), "/x/y.tex");
        if !cfg!(windows) {
            assert_eq!(normalize("sub\\a.tex", base), "/proj/sub/a.tex");
        }
    }

    #[test]
    fn unit_and_magnification_scale_the_coordinates() {
        let text = sample().replace("Unit:1", "Unit:2");
        let idx = SyncIndex::parse(text.as_bytes()).unwrap();
        let r = idx
            .forward(Path::new("/proj/main.tex"), 4, Path::new("/proj"))
            .unwrap();
        assert!(close(r.rects[0].x, 144.0), "{:?}", r.rects[0]);
    }
}
