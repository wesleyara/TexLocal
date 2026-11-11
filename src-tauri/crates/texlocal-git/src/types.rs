use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Error {
    /// Repositório em merge/rebase/etc.: operações automáticas não devem rodar.
    Busy(String),
    /// Caminho fora do projeto ou inválido.
    BadPath(String),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Busy(s) | Self::BadPath(s) | Self::Other(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for Error {}

impl From<git2::Error> for Error {
    fn from(e: git2::Error) -> Self {
        Self::Other(e.message().to_owned())
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Other(e.to_string())
    }
}
impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Other(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TagInfo {
    /// Nome da referência sem `refs/tags/` (`texlocal/<slug>`).
    pub name: String,
    pub label: String,
    pub description: String,
    pub target: String,
    pub time: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    pub id: String,
    pub short: String,
    pub parent: Option<String>,
    pub summary: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub time: i64,
    pub files_changed: u32,
    pub checkpoint: bool,
    pub tags: Vec<TagInfo>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatusEntry {
    pub path: String,
    /// `added`, `modified`, `deleted`, `renamed` ou `conflicted`.
    pub status: String,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepoStatus {
    pub initialized: bool,
    /// `clean`, `merge`, `rebase`, `cherrypick`, `revert`, `bisect`, `other`.
    pub state: String,
    pub branch: Option<String>,
    pub entries: Vec<StatusEntry>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LogQuery {
    pub skip: usize,
    pub limit: usize,
    pub named_only: bool,
    pub path: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub path: String,
    pub status: String,
    pub additions: u32,
    pub deletions: u32,
    pub binary: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileSides {
    pub path: String,
    pub binary: bool,
    pub old_text: Option<String>,
    pub new_text: Option<String>,
    /// Binários pequenos (imagens) em base64, para a prévia antes/depois.
    pub old_data: Option<String>,
    pub new_data: Option<String>,
    pub mime: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    /// Checkpoint criado antes de restaurar (se havia mudanças).
    pub checkpoint: Option<CommitInfo>,
    /// Commit "Restaurado de ..." (ausente se nada mudou).
    pub commit: Option<CommitInfo>,
    /// Arquivos tocados (relativos à raiz), para recarregar buffers abertos.
    pub touched: Vec<String>,
}
