//! Instalador do TeX Live gerenciado: download retomável do `install-tl`, verificação de
//! checksum, instalação em diretório temporário e movimentação atômica ao final.
//!
//! Limitação de segurança: o `install-tl` é verificado por SHA-512 obtido do espelho (integridade
//! de transporte); a assinatura GPG do TeX Live não é verificada aqui.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha512};
use ureq::ResponseExt;

use crate::detect::find_executable;
use crate::profile::Profile;
use crate::tlmgr::{Tlmgr, TlmgrOp};
use crate::{disk, paths, process};

/// Redireciona geograficamente para um espelho próximo.
pub const DEFAULT_MIRROR: &str = "https://mirror.ctan.org/systems/texlive/tlnet";

/// Espelhos tentados, em ordem, quando o anterior falha.
pub const FALLBACK_MIRRORS: [&str; 3] = [
    "https://ctan.math.illinois.edu/systems/texlive/tlnet",
    "https://mirrors.mit.edu/CTAN/systems/texlive/tlnet",
    "https://ftp.fau.de/ctan/systems/texlive/tlnet",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Preflight,
    Download,
    Verify,
    Extract,
    Install,
    Finalize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallEvent {
    pub phase: Phase,
    /// 0.0..=1.0 quando conhecido.
    pub progress: Option<f32>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallErrorKind {
    Network,
    Checksum,
    Extract,
    InstallFailed,
    Cancelled,
    DiskSpace,
    AlreadyInstalled,
    Io,
}

impl InstallErrorKind {
    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::Network => "installer.error.network",
            Self::Checksum => "installer.error.checksum",
            Self::Extract => "installer.error.extract",
            Self::InstallFailed => "installer.error.installFailed",
            Self::Cancelled => "installer.error.cancelled",
            Self::DiskSpace => "installer.error.diskSpace",
            Self::AlreadyInstalled => "installer.error.alreadyInstalled",
            Self::Io => "installer.error.io",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallError {
    pub kind: InstallErrorKind,
    pub i18n_key: &'static str,
    pub details: String,
}

impl InstallError {
    pub fn new(kind: InstallErrorKind, details: impl Into<String>) -> Self {
        err(kind, details)
    }
}

fn err(kind: InstallErrorKind, details: impl Into<String>) -> InstallError {
    InstallError {
        kind,
        i18n_key: kind.i18n_key(),
        details: details.into(),
    }
}

fn io_err(e: std::io::Error) -> InstallError {
    err(InstallErrorKind::Io, e.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub year: u16,
    pub tree: PathBuf,
    pub bin_dir: PathBuf,
}

/// Nome do pacote do instalador para o SO atual.
pub fn archive_name() -> &'static str {
    if cfg!(windows) {
        "install-tl.zip"
    } else {
        "install-tl-unx.tar.gz"
    }
}

/// Lista de espelhos a tentar: o fixado (se houver), o padrão e os de reserva, sem repetições.
pub fn mirror_candidates(pinned: Option<&str>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let all = pinned
        .into_iter()
        .chain(std::iter::once(DEFAULT_MIRROR))
        .chain(FALLBACK_MIRRORS);
    for m in all {
        let m = m.trim().trim_end_matches('/').to_string();
        if !m.is_empty() && !out.contains(&m) {
            out.push(m);
        }
    }
    out
}

/// Extrai o hash de um arquivo `.sha512` ("<128 hex>  nome").
pub fn parse_sha512_file(text: &str) -> Option<String> {
    let token = text.split_whitespace().next()?;
    (token.len() == 128 && token.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| token.to_lowercase())
}

pub fn sha512_file(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha512::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Ano de release lido de `tlpkg/TeXLive/TLConfig.pm` (`$ReleaseYear = 2026;`).
pub fn parse_release_year(tlconfig: &str) -> Option<u16> {
    let re = Regex::new(r"ReleaseYear\s*=\s*(\d{4})").expect("regex");
    re.captures(tlconfig)?[1].parse().ok()
}

/// Progresso das linhas do install-tl, como `Installing [0012/0233, ...]: pacote`.
pub fn parse_install_progress(line: &str) -> Option<(u32, u32)> {
    let re = Regex::new(r"^\s*(?:Installing\s+)?\[\s*(\d+)\s*/\s*(\d+)").expect("regex");
    let caps = re.captures(line)?;
    Some((caps[1].parse().ok()?, caps[2].parse().ok()?))
}

/// Espaço necessário (bytes) para o perfil: tamanho instalado + 25% de folga para downloads.
pub fn required_bytes(profile: Profile) -> u64 {
    profile.approx_size_mb() * 1024 * 1024 * 5 / 4
}

fn agent() -> ureq::Agent {
    // Proxy HTTP(S): lido das variáveis de ambiente (HTTPS_PROXY/ALL_PROXY) pelo ureq.
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .build()
        .into()
}

fn net_err(e: impl std::fmt::Display) -> InstallError {
    err(InstallErrorKind::Network, e.to_string())
}

/// Baixa o texto de `url`; devolve também a URL final (após redirecionamentos).
fn fetch_text(agent: &ureq::Agent, url: &str) -> Result<(String, String), InstallError> {
    let mut resp = agent.get(url).call().map_err(net_err)?;
    let final_url = resp.get_uri().to_string();
    let text = resp.body_mut().read_to_string().map_err(net_err)?;
    Ok((text, final_url))
}

/// Download retomável: continua de `<dest>.part` com `Range`; ao concluir renomeia para `dest`.
pub fn download_resumable(
    agent: &ureq::Agent,
    url: &str,
    dest: &Path,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<(), InstallError> {
    let part = PathBuf::from(format!("{}.part", dest.display()));
    let mut attempt = 0;
    loop {
        attempt += 1;
        let existing = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        let mut req = agent.get(url);
        if existing > 0 {
            req = req.header("Range", format!("bytes={existing}-"));
        }
        let mut resp = match req.call() {
            Ok(r) => r,
            // Arquivo parcial maior/igual ao remoto: recomeça do zero uma vez.
            Err(ureq::Error::StatusCode(416)) if attempt == 1 => {
                let _ = fs::remove_file(&part);
                continue;
            }
            Err(e) => return Err(net_err(e)),
        };

        let resumed = resp.status().as_u16() == 206;
        let offset = if resumed { existing } else { 0 };
        let total = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .map(|len| len + offset);

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(!resumed)
            .append(resumed)
            .open(&part)
            .map_err(io_err)?;

        let mut done = offset;
        let mut reader = resp.body_mut().as_reader();
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err(err(InstallErrorKind::Cancelled, ""));
            }
            let n = reader.read(&mut buf).map_err(net_err)?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n]).map_err(io_err)?;
            done += n as u64;
            on_progress(done, total);
        }
        file.flush().map_err(io_err)?;
        drop(file);
        if let Some(total) = total {
            if done < total {
                return Err(net_err("download interrompido antes do fim"));
            }
        }
        fs::rename(&part, dest).map_err(io_err)?;
        return Ok(());
    }
}

/// Extrai `.zip` ou `.tar.gz` em `dest` (entradas com `..` são recusadas pelas bibliotecas).
pub fn extract_archive(archive: &Path, dest: &Path) -> Result<(), InstallError> {
    let extract_err = |e: &dyn std::fmt::Display| err(InstallErrorKind::Extract, e.to_string());
    fs::create_dir_all(dest).map_err(io_err)?;
    let file = File::open(archive).map_err(io_err)?;
    if archive.extension().is_some_and(|e| e == "zip") {
        let mut zip = zip::ZipArchive::new(file).map_err(|e| extract_err(&e))?;
        zip.extract(dest).map_err(|e| extract_err(&e))
    } else {
        let gz = flate2::read::GzDecoder::new(file);
        tar::Archive::new(gz)
            .unpack(dest)
            .map_err(|e| extract_err(&e))
    }
}

/// Encontra a pasta do instalador (a que contém o script `install-tl`) em até 1 nível.
pub fn find_install_root(dir: &Path) -> Option<PathBuf> {
    if dir.join("install-tl").is_file() {
        return Some(dir.to_path_buf());
    }
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.join("install-tl").is_file())
}

pub struct Installer {
    root: PathBuf,
    pinned_mirror: Option<String>,
}

impl Installer {
    pub fn new(root: PathBuf, pinned_mirror: Option<String>) -> Self {
        Self {
            root,
            pinned_mirror,
        }
    }

    fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    /// Baixa e verifica o instalador, tentando os espelhos em ordem. Devolve (arquivo, base resolvida).
    fn fetch_installer(
        &self,
        cancel: &AtomicBool,
        emit: &mut dyn FnMut(Phase, Option<f32>, String),
    ) -> Result<(PathBuf, String), InstallError> {
        let agent = agent();
        let name = archive_name();
        let dest = self.cache_dir().join(name);
        let mut last = err(InstallErrorKind::Network, "nenhum espelho disponível");

        for mirror in mirror_candidates(self.pinned_mirror.as_deref()) {
            if cancel.load(Ordering::SeqCst) {
                return Err(err(InstallErrorKind::Cancelled, ""));
            }
            emit(Phase::Download, None, mirror.clone());
            let attempt = (|| -> Result<(PathBuf, String), InstallError> {
                let sha_url = format!("{mirror}/{name}.sha512");
                let (sha_text, final_url) = fetch_text(&agent, &sha_url)?;
                let expected = parse_sha512_file(&sha_text)
                    .ok_or_else(|| err(InstallErrorKind::Checksum, "arquivo .sha512 inválido"))?;
                // Base resolvida (espelho geográfico escolhido): usada em todo o restante.
                let resolved = final_url
                    .strip_suffix(&format!("/{name}.sha512"))
                    .unwrap_or(&mirror)
                    .to_string();

                download_resumable(
                    &agent,
                    &format!("{resolved}/{name}"),
                    &dest,
                    cancel,
                    &mut |done, total| {
                        let p = total.map(|t| done as f32 / t.max(1) as f32);
                        emit(Phase::Download, p, resolved.clone());
                    },
                )?;
                emit(Phase::Verify, None, String::new());
                let actual = sha512_file(&dest).map_err(io_err)?;
                if actual != expected {
                    let _ = fs::remove_file(&dest);
                    return Err(err(
                        InstallErrorKind::Checksum,
                        format!("esperado {expected}, obtido {actual}"),
                    ));
                }
                Ok((dest.clone(), resolved))
            })();
            match attempt {
                Ok(ok) => return Ok(ok),
                Err(e) if e.kind == InstallErrorKind::Cancelled => return Err(e),
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    /// Instala o TeX Live do `profile`. Instala em diretório temporário e só ao final move para
    /// `texlive/<ano>`, de modo que uma falha nunca deixa uma instalação corrompida.
    pub fn install(
        &self,
        profile: Profile,
        cancel: &AtomicBool,
        on_event: &mut dyn FnMut(InstallEvent),
    ) -> Result<InstallResult, InstallError> {
        let mut emit = |phase: Phase, progress: Option<f32>, message: String| {
            on_event(InstallEvent {
                phase,
                progress,
                message,
            });
        };

        emit(Phase::Preflight, None, String::new());
        fs::create_dir_all(self.cache_dir()).map_err(io_err)?;
        if let Some(free) = disk::free_space(&self.root) {
            let needed = required_bytes(profile);
            if free < needed {
                return Err(err(
                    InstallErrorKind::DiskSpace,
                    format!("necessário {needed} bytes, livre {free}"),
                ));
            }
        }

        let (archive, repo) = self.fetch_installer(cancel, &mut emit)?;

        emit(Phase::Extract, None, String::new());
        let extract_dir = self.cache_dir().join("install-tl-extract");
        let _ = fs::remove_dir_all(&extract_dir);
        extract_archive(&archive, &extract_dir)?;
        let installer_dir = find_install_root(&extract_dir).ok_or_else(|| {
            err(
                InstallErrorKind::Extract,
                "script install-tl não encontrado",
            )
        })?;
        let tlconfig = fs::read_to_string(
            installer_dir
                .join("tlpkg")
                .join("TeXLive")
                .join("TLConfig.pm"),
        )
        .map_err(io_err)?;
        let year = parse_release_year(&tlconfig)
            .ok_or_else(|| err(InstallErrorKind::Extract, "ano do TeX Live não encontrado"))?;

        let final_tree = paths::managed_tree(&self.root, year);
        if final_tree.exists() {
            return Err(err(
                InstallErrorKind::AlreadyInstalled,
                final_tree.display().to_string(),
            ));
        }
        let partial = self.root.join("texlive").join(format!(".{year}.partial"));
        let _ = fs::remove_dir_all(&partial);
        fs::create_dir_all(partial.parent().expect("tem pai")).map_err(io_err)?;

        let profile_file = self.cache_dir().join("texlocal.profile");
        fs::write(&profile_file, profile.install_profile(&partial)).map_err(io_err)?;

        emit(Phase::Install, None, String::new());
        let result = self.run_install_tl(&installer_dir, &profile_file, &repo, cancel, &mut emit);
        let _ = fs::remove_dir_all(&extract_dir);
        if let Err(e) = result {
            let _ = fs::remove_dir_all(&partial);
            return Err(e);
        }

        emit(Phase::Finalize, None, String::new());
        fs::rename(&partial, &final_tree).map_err(io_err)?;
        let bin_dir = paths::managed_bin_dir(&final_tree);

        if let Some(tlmgr_bin) = find_executable(&bin_dir, "tlmgr") {
            let tlmgr = Tlmgr::new(tlmgr_bin, false);
            let pkgs = profile
                .post_install_packages()
                .iter()
                .map(|s| (*s).to_string())
                .collect();
            tlmgr
                .run(&TlmgrOp::Install(pkgs), cancel, &mut |_| {})
                .map_err(|e| err(InstallErrorKind::InstallFailed, e.details))?;
        }
        Ok(InstallResult {
            year,
            tree: final_tree,
            bin_dir,
        })
    }

    fn run_install_tl(
        &self,
        installer_dir: &Path,
        profile_file: &Path,
        repo: &str,
        cancel: &AtomicBool,
        emit: &mut dyn FnMut(Phase, Option<f32>, String),
    ) -> Result<(), InstallError> {
        let mut cmd = if cfg!(windows) {
            process::prepare(&installer_dir.join("install-tl-windows.bat"))
        } else {
            let mut c = process::prepare(Path::new("perl"));
            c.arg("install-tl");
            c
        };
        cmd.current_dir(installer_dir)
            .arg("-profile")
            .arg(profile_file)
            .arg("-repository")
            .arg(repo);

        let mut tail: Vec<String> = Vec::new();
        let output = process::run_streaming(cmd, cancel, &mut |line| {
            if let Some((cur, total)) = parse_install_progress(line) {
                emit(
                    Phase::Install,
                    Some(cur as f32 / total.max(1) as f32),
                    line.trim().to_string(),
                );
            }
            tail.push(line.to_string());
            if tail.len() > 40 {
                tail.remove(0);
            }
        })
        .map_err(|e| err(InstallErrorKind::InstallFailed, e.to_string()))?;

        if output.cancelled {
            return Err(err(InstallErrorKind::Cancelled, ""));
        }
        if output.status.success() {
            Ok(())
        } else {
            Err(err(
                InstallErrorKind::InstallFailed,
                format!("{}\n{}", tail.join("\n"), output.stderr),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    /// Servidor HTTP mínimo com suporte a `Range`; registra os cabeçalhos Range recebidos.
    fn serve(data: Vec<u8>, requests: usize) -> (String, Arc<Mutex<Vec<Option<u64>>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/file.bin", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_t = Arc::clone(&seen);
        thread::spawn(move || {
            for _ in 0..requests {
                let (mut stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut range = None;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = line.to_lowercase().strip_prefix("range: bytes=") {
                        range = v.trim().trim_end_matches('-').parse::<u64>().ok();
                    }
                }
                seen_t.lock().unwrap().push(range);
                let (status, body, extra) = match range {
                    Some(start) if (start as usize) < data.len() => (
                        "206 Partial Content",
                        &data[start as usize..],
                        format!(
                            "Content-Range: bytes {start}-{}/{}\r\n",
                            data.len() - 1,
                            data.len()
                        ),
                    ),
                    Some(_) => ("416 Range Not Satisfiable", &data[..0], String::new()),
                    None => ("200 OK", &data[..], String::new()),
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(body);
            }
        });
        (url, seen)
    }

    fn sample(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    fn mirror_list_puts_pinned_first_without_duplicates() {
        let list = mirror_candidates(Some("https://example.org/tlnet/"));
        assert_eq!(list[0], "https://example.org/tlnet");
        assert_eq!(list[1], DEFAULT_MIRROR);
        assert_eq!(list.len(), 2 + FALLBACK_MIRRORS.len());
        let again = mirror_candidates(Some(DEFAULT_MIRROR));
        assert_eq!(again.len(), 1 + FALLBACK_MIRRORS.len());
        assert_eq!(mirror_candidates(None)[0], DEFAULT_MIRROR);
    }

    #[test]
    fn parses_checksum_file_and_release_year() {
        let hash = "a".repeat(128);
        assert_eq!(
            parse_sha512_file(&format!("{hash}  install-tl.zip\n")),
            Some(hash)
        );
        assert_eq!(parse_sha512_file("curto  x"), None);
        assert_eq!(parse_release_year("our $ReleaseYear = 2026;\n"), Some(2026));
        assert_eq!(parse_release_year("nada"), None);
    }

    #[test]
    fn parses_install_progress_lines() {
        assert_eq!(
            parse_install_progress("Installing [0012/0233, time/total: 00:03/01:10]: foo"),
            Some((12, 233))
        );
        assert_eq!(parse_install_progress("[ 3/ 10] x"), Some((3, 10)));
        assert_eq!(parse_install_progress("Installing TeX Live"), None);
    }

    #[test]
    fn computes_sha512() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("a");
        fs::write(&f, b"abc").unwrap();
        // SHA-512("abc") conhecido (FIPS 180-2).
        assert!(sha512_file(&f).unwrap().starts_with("ddaf35a193617aba"));
    }

    #[test]
    fn required_space_grows_with_profile() {
        assert!(required_bytes(Profile::Minimal) < required_bytes(Profile::Recommended));
        assert!(required_bytes(Profile::Recommended) < required_bytes(Profile::Full));
    }

    #[test]
    fn downloads_from_scratch() {
        let data = sample(200_000);
        let (url, seen) = serve(data.clone(), 1);
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("f.bin");
        let mut last = (0, None);
        download_resumable(
            &agent(),
            &url,
            &dest,
            &AtomicBool::new(false),
            &mut |d, t| last = (d, t),
        )
        .unwrap();
        assert_eq!(fs::read(&dest).unwrap(), data);
        assert_eq!(last, (200_000, Some(200_000)));
        assert_eq!(*seen.lock().unwrap(), vec![None]);
        assert!(!tmp.path().join("f.bin.part").exists());
    }

    #[test]
    fn resumes_a_partial_download_with_range() {
        let data = sample(100_000);
        let (url, seen) = serve(data.clone(), 1);
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("f.bin");
        fs::write(tmp.path().join("f.bin.part"), &data[..40_000]).unwrap();
        download_resumable(
            &agent(),
            &url,
            &dest,
            &AtomicBool::new(false),
            &mut |_, _| {},
        )
        .unwrap();
        assert_eq!(fs::read(&dest).unwrap(), data);
        assert_eq!(*seen.lock().unwrap(), vec![Some(40_000)]);
    }

    #[test]
    fn restarts_when_partial_file_is_larger_than_remote() {
        let data = sample(1_000);
        let (url, seen) = serve(data.clone(), 2);
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("f.bin");
        fs::write(tmp.path().join("f.bin.part"), sample(5_000)).unwrap();
        download_resumable(
            &agent(),
            &url,
            &dest,
            &AtomicBool::new(false),
            &mut |_, _| {},
        )
        .unwrap();
        assert_eq!(fs::read(&dest).unwrap(), data);
        assert_eq!(*seen.lock().unwrap(), vec![Some(5_000), None]);
    }

    #[test]
    fn cancelled_download_keeps_partial_file_and_reports_cancel() {
        let data = sample(500_000);
        let (url, _) = serve(data, 1);
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("f.bin");
        let e = download_resumable(
            &agent(),
            &url,
            &dest,
            &AtomicBool::new(true),
            &mut |_, _| {},
        )
        .unwrap_err();
        assert_eq!(e.kind, InstallErrorKind::Cancelled);
        assert!(!dest.exists());
    }

    #[test]
    fn network_failure_is_a_network_error() {
        let tmp = tempfile::tempdir().unwrap();
        let e = download_resumable(
            &agent(),
            "http://127.0.0.1:1/x",
            &tmp.path().join("x"),
            &AtomicBool::new(false),
            &mut |_, _| {},
        )
        .unwrap_err();
        assert_eq!(e.kind, InstallErrorKind::Network);
        assert_eq!(e.i18n_key, "installer.error.network");
    }

    fn make_tar_gz(path: &Path, entries: &[(&str, &[u8])]) {
        let gz =
            flate2::write::GzEncoder::new(File::create(path).unwrap(), flate2::Compression::fast());
        let mut b = tar::Builder::new(gz);
        for (name, body) in entries {
            let mut h = tar::Header::new_gnu();
            h.set_size(body.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            b.append_data(&mut h, name, *body).unwrap();
        }
        b.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn extracts_tar_gz_and_finds_install_root() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("a.tar.gz");
        make_tar_gz(
            &archive,
            &[
                ("install-tl-2026/install-tl", b"#!perl"),
                (
                    "install-tl-2026/tlpkg/TeXLive/TLConfig.pm",
                    b"$ReleaseYear = 2026;",
                ),
            ],
        );
        let out = tmp.path().join("out");
        extract_archive(&archive, &out).unwrap();
        let root = find_install_root(&out).unwrap();
        assert!(root.ends_with("install-tl-2026"));
        let cfg = fs::read_to_string(root.join("tlpkg/TeXLive/TLConfig.pm")).unwrap();
        assert_eq!(parse_release_year(&cfg), Some(2026));
    }

    #[test]
    fn extracts_zip() {
        use zip::write::SimpleFileOptions;
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("a.zip");
        {
            let mut z = zip::ZipWriter::new(File::create(&archive).unwrap());
            z.start_file("install-tl-2026/install-tl", SimpleFileOptions::default())
                .unwrap();
            z.write_all(b"x").unwrap();
            z.finish().unwrap();
        }
        let out = tmp.path().join("out");
        extract_archive(&archive, &out).unwrap();
        assert!(find_install_root(&out).is_some());
    }

    #[test]
    fn corrupt_archive_is_an_extract_error() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("a.tar.gz");
        fs::write(&archive, b"isto nao e gzip").unwrap();
        let e = extract_archive(&archive, &tmp.path().join("o")).unwrap_err();
        assert_eq!(e.kind, InstallErrorKind::Extract);
    }
}

/// Teste de ponta a ponta contra o CTAN real (baixa centenas de MB): `cargo test -- --ignored`.
#[cfg(test)]
mod real_network {
    use super::*;

    #[test]
    #[ignore = "baixa e instala o TeX Live de verdade"]
    fn installs_minimal_profile_end_to_end() {
        let tmp = tempfile::tempdir().unwrap();
        // TEXLOCAL_TEST_ROOT mantém a instalação para inspeção manual.
        let root = std::env::var_os("TEXLOCAL_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| tmp.path().to_path_buf());
        let installer = Installer::new(root.clone(), None);
        let mut last_phase = None;
        let result = installer
            .install(Profile::Minimal, &AtomicBool::new(false), &mut |e| {
                if last_phase != Some(e.phase) || e.progress.is_none() {
                    println!("{:?} {:?} {}", e.phase, e.progress, e.message);
                }
                last_phase = Some(e.phase);
            })
            .unwrap_or_else(|e| panic!("{:?}: {}", e.kind, e.details));
        println!("instalado: {result:?}");
        let dist =
            crate::detect::probe(&result.bin_dir).expect("pdflatex funcional após mover a árvore");
        println!("{dist:?}");
        assert_eq!(dist.year, Some(result.year));
        assert!(dist.has_tlmgr);
        assert!(dist.has_latexmk);
        assert!(!tmp
            .path()
            .join("texlive")
            .join(format!(".{}.partial", result.year))
            .exists());
    }
}

/// Valida o serviço tlmgr e os parsers contra uma instalação real (use TEXLOCAL_TEST_ROOT).
#[cfg(test)]
mod real_tlmgr {
    use crate::{detect, logparse, paths, tlmgr};
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;

    #[test]
    #[ignore = "requer TEXLOCAL_TEST_ROOT com uma instalação feita pelo teste de ponta a ponta"]
    fn tlmgr_service_against_installed_tree() {
        let root =
            PathBuf::from(std::env::var_os("TEXLOCAL_TEST_ROOT").expect("TEXLOCAL_TEST_ROOT"));
        let bin = paths::managed_bin_dir(&paths::managed_tree(&root, 2026));
        let tlmgr_bin = detect::find_executable(&bin, "tlmgr").expect("tlmgr");
        let svc = tlmgr::Tlmgr::new(tlmgr_bin, false);
        let cancel = AtomicBool::new(false);

        let list = svc
            .run(&tlmgr::TlmgrOp::ListInstalled, &cancel, &mut |_| {})
            .unwrap();
        let installed = tlmgr::parse_installed(&list);
        assert!(
            installed.iter().any(|p| p == "latexmk"),
            "latexmk deve estar instalado"
        );
        assert!(installed.iter().any(|p| p == "collection-langportuguese"));

        let out = svc
            .run(
                &tlmgr::TlmgrOp::SearchFile("siunitx.sty".into()),
                &cancel,
                &mut |_| {},
            )
            .unwrap();
        let pkgs = logparse::parse_search_output(&out, "siunitx.sty");
        assert_eq!(
            logparse::choose_package(&pkgs).map(String::as_str),
            Some("siunitx")
        );

        let mut progress = 0;
        svc.run(
            &tlmgr::TlmgrOp::Install(vec!["siunitx".into()]),
            &cancel,
            &mut |_| progress += 1,
        )
        .unwrap();
        println!("eventos de progresso do install: {progress}");
        let installed = tlmgr::parse_installed(
            &svc.run(&tlmgr::TlmgrOp::ListInstalled, &cancel, &mut |_| {})
                .unwrap(),
        );
        assert!(installed.iter().any(|p| p == "siunitx"));

        let err = svc
            .run(
                &tlmgr::TlmgrOp::Install(vec!["pacote-que-nao-existe-xyz".into()]),
                &cancel,
                &mut |_| {},
            )
            .unwrap_err();
        println!(
            "erro p/ pacote inexistente: {:?} / {}",
            err.kind,
            err.details.trim()
        );
    }
}
