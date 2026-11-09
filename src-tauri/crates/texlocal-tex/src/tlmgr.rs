//! Camada de serviço sobre o `tlmgr`: operações, fila serializada, progresso e cancelamento.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use regex::Regex;
use serde::Serialize;

use crate::command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlmgrOp {
    Install(Vec<String>),
    Remove(Vec<String>),
    UpdateAll,
    UpdateList,
    SearchFile(String),
    Info(String),
    ListInstalled,
}

impl TlmgrOp {
    pub fn args(&self) -> Vec<String> {
        let s = |v: &[&str]| v.iter().map(|x| (*x).to_string()).collect::<Vec<_>>();
        match self {
            Self::Install(p) => [s(&["install"]), p.clone()].concat(),
            Self::Remove(p) => [s(&["remove"]), p.clone()].concat(),
            Self::UpdateAll => s(&["update", "--all"]),
            Self::UpdateList => s(&["update", "--list"]),
            Self::SearchFile(f) => [s(&["search", "--global", "--file"]), vec![f.clone()]].concat(),
            Self::Info(p) => s(&["info", p]),
            Self::ListInstalled => s(&["info", "--only-installed", "--data", "name"]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub current: u32,
    pub total: u32,
    pub action: String,
    pub package: String,
}

/// Interpreta linhas como `[ 3/12, 00:01/00:05] install: siunitx [1234k]`.
pub fn parse_progress(line: &str) -> Option<Progress> {
    let re = Regex::new(r"^\[\s*(\d+)/(\d+)[^\]]*\]\s+(\w+):\s+(\S+)").expect("regex");
    let caps = re.captures(line.trim())?;
    Some(Progress {
        current: caps[1].parse().ok()?,
        total: caps[2].parse().ok()?,
        action: caps[3].to_string(),
        package: caps[4].to_string(),
    })
}

/// Pacotes listados por `tlmgr info --only-installed --data name`.
pub fn parse_installed(output: &str) -> Vec<String> {
    output
        .lines()
        .map(|l| l.split(',').next().unwrap_or("").trim())
        .filter(|l| !l.is_empty() && !l.contains(' ') && !l.starts_with("tlmgr"))
        .map(str::to_string)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TlmgrErrorKind {
    Offline,
    PackageNotFound,
    PermissionDenied,
    Cancelled,
    Unknown,
}

impl TlmgrErrorKind {
    /// Chave de tradução (frontend) da mensagem amigável.
    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::Offline => "tlmgr.error.offline",
            Self::PackageNotFound => "tlmgr.error.packageNotFound",
            Self::PermissionDenied => "tlmgr.error.permissionDenied",
            Self::Cancelled => "tlmgr.error.cancelled",
            Self::Unknown => "tlmgr.error.unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TlmgrError {
    pub kind: TlmgrErrorKind,
    pub i18n_key: &'static str,
    /// Saída original do tlmgr, para o log.
    pub details: String,
}

pub fn classify_error(output: &str) -> TlmgrErrorKind {
    let o = output.to_lowercase();
    let any = |needles: &[&str]| needles.iter().any(|n| o.contains(n));
    if any(&[
        "could not resolve",
        "couldn't resolve",
        "cannot contact",
        "connection refused",
        "connection timed out",
        "unable to download",
        "network is unreachable",
        "could not connect",
        "failed to download",
    ]) {
        TlmgrErrorKind::Offline
    } else if any(&["permission denied", "not writable", "access is denied"]) {
        TlmgrErrorKind::PermissionDenied
    } else if any(&[
        "not present in repository",
        "unknown package",
        "no such package",
    ]) || (o.contains("package") && o.contains("not found"))
    {
        TlmgrErrorKind::PackageNotFound
    } else {
        TlmgrErrorKind::Unknown
    }
}

fn error(kind: TlmgrErrorKind, details: String) -> TlmgrError {
    TlmgrError {
        kind,
        i18n_key: kind.i18n_key(),
        details,
    }
}

/// Encerra o processo e seus descendentes (o tlmgr é um wrapper que dispara outros processos).
fn kill_tree(child: &mut Child) {
    let pid = child.id().to_string();
    #[cfg(windows)]
    {
        let _ = command(Path::new("taskkill"))
            .args(["/PID", &pid, "/T", "/F"])
            .output();
    }
    #[cfg(unix)]
    {
        // O processo foi iniciado em um grupo próprio (ver `run`).
        let _ = std::process::Command::new("kill")
            .args(["-KILL", "--", &format!("-{pid}")])
            .output();
    }
    let _ = child.kill();
}

pub struct Tlmgr {
    bin: PathBuf,
    usermode: bool,
    /// Uma operação por vez.
    queue: Mutex<()>,
}

impl Tlmgr {
    pub fn new(bin: PathBuf, usermode: bool) -> Self {
        Self {
            bin,
            usermode,
            queue: Mutex::new(()),
        }
    }

    /// Executa a operação (bloqueia até as anteriores terminarem). Devolve a saída padrão.
    pub fn run(
        &self,
        op: &TlmgrOp,
        cancel: &AtomicBool,
        on_progress: &mut dyn FnMut(Progress),
    ) -> Result<String, TlmgrError> {
        let _turn = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        if cancel.load(Ordering::SeqCst) {
            return Err(error(TlmgrErrorKind::Cancelled, String::new()));
        }

        let mut cmd = command(&self.bin);
        if self.usermode {
            cmd.arg("--usermode");
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        let mut child = cmd
            .args(op.args())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| error(TlmgrErrorKind::Unknown, e.to_string()))?;

        let stdout = child.stdout.take().expect("stdout piped");
        let mut stderr = child.stderr.take().expect("stderr piped");
        let child = Arc::new(Mutex::new(child));
        let finished = Arc::new(AtomicBool::new(false));

        let err_reader = thread::spawn(move || {
            let mut buf = String::new();
            let _ = stderr.read_to_string(&mut buf);
            buf
        });

        let mut out = String::new();
        thread::scope(|scope| {
            // Vigia o pedido de cancelamento enquanto o processo roda.
            let child_w = Arc::clone(&child);
            let finished_w = Arc::clone(&finished);
            scope.spawn(move || {
                while !finished_w.load(Ordering::SeqCst) {
                    if cancel.load(Ordering::SeqCst) {
                        let mut c = child_w.lock().unwrap_or_else(|e| e.into_inner());
                        kill_tree(&mut c);
                        return;
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            });

            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(p) = parse_progress(&line) {
                    on_progress(p);
                }
                out.push_str(&line);
                out.push('\n');
            }
            finished.store(true, Ordering::SeqCst);
        });
        let cancelled = cancel.load(Ordering::SeqCst);

        let status = child
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .wait()
            .map_err(|e| error(TlmgrErrorKind::Unknown, e.to_string()))?;
        let err_text = err_reader.join().unwrap_or_default();

        if cancelled {
            return Err(error(TlmgrErrorKind::Cancelled, err_text));
        }
        if status.success() {
            Ok(out)
        } else {
            let details = format!("{out}{err_text}");
            Err(error(classify_error(&details), details))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// Cria um "tlmgr" falso (script) no diretório temporário.
    #[cfg(windows)]
    fn fake_tlmgr(dir: &std::path::Path, _body_unix: &str, body_win: &str) -> PathBuf {
        let p = dir.join("tlmgr.bat");
        std::fs::write(&p, format!("@echo off\r\n{body_win}\r\n")).unwrap();
        p
    }

    #[cfg(unix)]
    fn fake_tlmgr(dir: &std::path::Path, body_unix: &str, _body_win: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let p = dir.join("tlmgr");
        std::fs::write(&p, format!("#!/bin/sh\n{body_unix}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    #[test]
    fn builds_arguments_for_each_operation() {
        assert_eq!(
            TlmgrOp::Install(vec!["a".into(), "b".into()]).args(),
            ["install", "a", "b"]
        );
        assert_eq!(TlmgrOp::UpdateAll.args(), ["update", "--all"]);
        assert_eq!(
            TlmgrOp::SearchFile("x.sty".into()).args(),
            ["search", "--global", "--file", "x.sty"]
        );
        assert_eq!(TlmgrOp::Remove(vec!["a".into()]).args(), ["remove", "a"]);
    }

    #[test]
    fn parses_progress_lines() {
        let p = parse_progress("[ 3/12, 00:01/00:05] install: siunitx [1234k]").unwrap();
        assert_eq!((p.current, p.total), (3, 12));
        assert_eq!(p.action, "install");
        assert_eq!(p.package, "siunitx");
        assert!(parse_progress("tlmgr: nothing").is_none());
    }

    #[test]
    fn parses_installed_list() {
        let out = "tlmgr: package repository x\nabntex2\nsiunitx,\nsome words here\n";
        assert_eq!(parse_installed(out), vec!["abntex2", "siunitx"]);
    }

    #[test]
    fn classifies_errors_into_friendly_kinds() {
        assert_eq!(
            classify_error("tlmgr: could not resolve host mirror"),
            TlmgrErrorKind::Offline
        );
        assert_eq!(
            classify_error("install: Permission denied"),
            TlmgrErrorKind::PermissionDenied
        );
        assert_eq!(
            classify_error("package foo not found in repository"),
            TlmgrErrorKind::PackageNotFound
        );
        assert_eq!(classify_error("???"), TlmgrErrorKind::Unknown);
        assert_eq!(TlmgrErrorKind::Offline.i18n_key(), "tlmgr.error.offline");
    }

    #[test]
    fn runs_reports_progress_and_returns_stdout() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = fake_tlmgr(
            tmp.path(),
            "echo '[1/2, 00:00/00:01] install: foo [10k]'\necho '[2/2, 00:00/00:01] install: bar [5k]'",
            "echo [1/2, 00:00/00:01] install: foo [10k]\r\necho [2/2, 00:00/00:01] install: bar [5k]",
        );
        let tlmgr = Tlmgr::new(bin, false);
        let mut seen = Vec::new();
        let out = tlmgr
            .run(
                &TlmgrOp::Install(vec!["foo".into(), "bar".into()]),
                &AtomicBool::new(false),
                &mut |p| seen.push((p.current, p.package)),
            )
            .unwrap();
        assert!(out.contains("install: bar"));
        assert_eq!(seen, vec![(1, "foo".to_string()), (2, "bar".to_string())]);
    }

    #[test]
    fn failure_is_translated_to_a_friendly_error() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = fake_tlmgr(
            tmp.path(),
            "echo 'tlmgr: could not resolve host' >&2\nexit 1",
            "echo tlmgr: could not resolve host 1>&2\r\nexit /b 1",
        );
        let err = Tlmgr::new(bin, false)
            .run(&TlmgrOp::UpdateAll, &AtomicBool::new(false), &mut |_| {})
            .unwrap_err();
        assert_eq!(err.kind, TlmgrErrorKind::Offline);
        assert_eq!(err.i18n_key, "tlmgr.error.offline");
        assert!(err.details.contains("could not resolve"));
    }

    #[test]
    fn running_operation_can_be_cancelled() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = fake_tlmgr(tmp.path(), "sleep 20", "ping -n 20 127.0.0.1 >nul");
        let tlmgr = Tlmgr::new(bin, false);
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            flag.store(true, Ordering::SeqCst);
        });
        let start = Instant::now();
        let err = tlmgr
            .run(&TlmgrOp::UpdateAll, &cancel, &mut |_| {})
            .unwrap_err();
        assert_eq!(err.kind, TlmgrErrorKind::Cancelled);
        assert!(start.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn already_cancelled_operation_does_not_start() {
        let tlmgr = Tlmgr::new(PathBuf::from("nao-existe"), false);
        let err = tlmgr
            .run(&TlmgrOp::UpdateAll, &AtomicBool::new(true), &mut |_| {})
            .unwrap_err();
        assert_eq!(err.kind, TlmgrErrorKind::Cancelled);
    }
}
