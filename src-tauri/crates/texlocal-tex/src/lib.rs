//! Gerenciamento do TeX Live: detecção, manifesto do projeto, `tlmgr` e utilitários.

pub mod compile;
pub mod detect;
pub mod diagnostics;
pub mod disk;
pub mod installer;
pub mod logparse;
pub mod manifest;
pub mod paths;
pub mod process;
pub mod profile;
pub mod reconcile;
pub mod synctex;
pub mod tlmgr;

use std::path::Path;
use std::process::Command;

/// Cria um `Command` que não abre janela de console no Windows.
pub(crate) fn command(program: &Path) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}
