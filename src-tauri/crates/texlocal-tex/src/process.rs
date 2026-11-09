//! Execução de processos externos com saída em streaming e cancelamento (mata a árvore inteira).

use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::command;

pub struct Output {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
    pub cancelled: bool,
}

/// Encerra o processo e seus descendentes (wrappers como `tlmgr.bat` e `perl` disparam filhos).
pub fn kill_tree(child: &mut Child) {
    let pid = child.id().to_string();
    #[cfg(windows)]
    {
        let _ = command(Path::new("taskkill"))
            .args(["/PID", &pid, "/T", "/F"])
            .output();
    }
    #[cfg(unix)]
    {
        // O processo foi iniciado em um grupo próprio (ver `run_streaming`).
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{pid}")])
            .output();
    }
    let _ = child.kill();
}

/// Prepara um `Command` para `program` sem janela de console (Windows) e em grupo próprio (Unix).
pub fn prepare(program: &Path) -> Command {
    #[allow(unused_mut)]
    let mut cmd = command(program);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd
}

/// Executa `cmd`, entregando cada linha da saída padrão a `on_line`. Se `cancel` for ativado,
/// mata o processo (e filhos) e devolve `cancelled = true`.
pub fn run_streaming(
    mut cmd: Command,
    cancel: &AtomicBool,
    on_line: &mut dyn FnMut(&str),
) -> std::io::Result<Output> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

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
            on_line(&line);
            out.push_str(&line);
            out.push('\n');
        }
        finished.store(true, Ordering::SeqCst);
    });
    let cancelled = cancel.load(Ordering::SeqCst);

    let status = child.lock().unwrap_or_else(|e| e.into_inner()).wait()?;
    Ok(Output {
        status,
        stdout: out,
        stderr: err_reader.join().unwrap_or_default(),
        cancelled,
    })
}
