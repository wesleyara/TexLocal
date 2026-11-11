//! Camada git do TexLocal (EP06): histórico amigável sobre um repositório git real.
//!
//! Conceitos da UI ↔ git: checkpoint = commit com mensagem gerada; versão nomeada = tag anotada
//! `texlocal/<slug>`; restaurar = novo commit (nunca reescreve histórico).

mod ops;
mod types;

pub use ops::*;
pub use types::*;
