use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use git2::{
    Commit, Delta, Diff, DiffOptions, IndexAddOption, ObjectType, Oid, Patch, Repository,
    RepositoryState, Signature, Sort, Status, StatusOptions, Tree, TreeWalkMode, TreeWalkResult,
};

use crate::types::*;

const TAG_PREFIX: &str = "texlocal/";
const CHECKPOINT_PREFIX: &str = "chore(checkpoint)";
/// Binários acima disso não vão para a prévia.
const MAX_PREVIEW_BYTES: usize = 8 * 1024 * 1024;

const GITATTRIBUTES: &str = "\
* text=auto eol=lf
*.pdf binary
*.png binary
*.jpg binary
*.jpeg binary
*.gif binary
*.eps binary
*.zip binary
*.docx binary
*.xlsx binary
*.pptx binary
*.ttf binary
*.otf binary
*.woff binary
*.woff2 binary
";

const GITIGNORE: &str = "\
# TexLocal
.texlocal/build/
.texlocal/recovery/

# Artefatos LaTeX
*.aux
*.log
*.out
*.toc
*.lof
*.lot
*.fls
*.fdb_latexmk
*.synctex.gz
*.synctex(busy)
*.bbl
*.blg
*.bcf
*.run.xml
*.nav
*.snm
*.vrb
*.idx
*.ilg
*.ind
*.glo
*.gls
*.glg
*.acn
*.acr
*.alg
*.xdv
*.dvi
*.thm
*.bak
*.spl
_minted*/
";

// ───────────────────────── abertura e init ─────────────────────────

pub fn is_repo(root: &Path) -> bool {
    Repository::open(root).is_ok()
}

fn open(root: &Path) -> Result<Repository> {
    Repository::open(root)
        .map_err(|e| Error::Other(format!("histórico não ativado: {}", e.message())))
}

fn signature(id: &Identity) -> Result<Signature<'static>> {
    Ok(Signature::now(&id.name, &id.email)?)
}

fn ensure_idle(repo: &Repository) -> Result<()> {
    match repo.state() {
        RepositoryState::Clean => Ok(()),
        s => Err(Error::Busy(format!(
            "o repositório está em andamento ({}); conclua ou cancele antes",
            state_name(s)
        ))),
    }
}

fn state_name(s: RepositoryState) -> &'static str {
    match s {
        RepositoryState::Clean => "clean",
        RepositoryState::Merge => "merge",
        RepositoryState::Revert | RepositoryState::RevertSequence => "revert",
        RepositoryState::CherryPick | RepositoryState::CherryPickSequence => "cherrypick",
        RepositoryState::Bisect => "bisect",
        RepositoryState::Rebase
        | RepositoryState::RebaseInteractive
        | RepositoryState::RebaseMerge => "rebase",
        _ => "other",
    }
}

/// Ativa o histórico: `git init`, `.gitattributes`/`.gitignore` padrão e o commit "Versão inicial".
/// Repositório já existente (criado pelo git CLI) é reaproveitado sem alterar seu conteúdo.
pub fn init(root: &Path, id: &Identity) -> Result<Option<CommitInfo>> {
    let existing = Repository::open(root).is_ok();
    let repo = Repository::init(root)?;
    if existing {
        return Ok(None);
    }
    let mut cfg = repo.config()?;
    cfg.set_bool("core.autocrlf", false)?;
    cfg.set_str("core.eol", "lf")?;
    for (name, body) in [(".gitattributes", GITATTRIBUTES), (".gitignore", GITIGNORE)] {
        let p = root.join(name);
        if !p.exists() {
            fs::write(p, body)?;
        }
    }
    let oid = commit_all(&repo, id, "Versão inicial", true)?;
    commit_info(
        &repo,
        oid.ok_or_else(|| Error::Other("commit inicial vazio".into()))?,
    )
    .map(Some)
}

// ───────────────────────── status ─────────────────────────

pub fn status(root: &Path) -> Result<RepoStatus> {
    let Ok(repo) = Repository::open(root) else {
        return Ok(RepoStatus {
            initialized: false,
            state: "clean".into(),
            branch: None,
            entries: Vec::new(),
        });
    };
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true);
    let statuses = repo.statuses(Some(&mut opts))?;
    let mut entries = Vec::new();
    for e in statuses.iter() {
        let Some(path) = e.path() else { continue };
        let s = e.status();
        if s.is_empty() || s.contains(Status::IGNORED) {
            continue;
        }
        let staged = s.intersects(
            Status::INDEX_NEW
                | Status::INDEX_MODIFIED
                | Status::INDEX_DELETED
                | Status::INDEX_RENAMED
                | Status::INDEX_TYPECHANGE,
        );
        let untracked = s.contains(Status::WT_NEW);
        let unstaged = untracked
            || s.intersects(
                Status::WT_MODIFIED
                    | Status::WT_DELETED
                    | Status::WT_TYPECHANGE
                    | Status::WT_RENAMED,
            );
        let status = if s.contains(Status::CONFLICTED) {
            "conflicted"
        } else if s.intersects(Status::INDEX_DELETED | Status::WT_DELETED) {
            "deleted"
        } else if s.intersects(Status::INDEX_NEW | Status::WT_NEW) {
            "added"
        } else if s.intersects(Status::INDEX_RENAMED | Status::WT_RENAMED) {
            "renamed"
        } else {
            "modified"
        };
        entries.push(StatusEntry {
            path: path.to_owned(),
            status: status.into(),
            staged,
            unstaged,
            untracked,
        });
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(str::to_owned))
        .or_else(|| {
            // HEAD "não nascido": branch inicial ainda sem commits.
            repo.find_reference("HEAD").ok().and_then(|r| {
                r.symbolic_target()
                    .map(|t| t.trim_start_matches("refs/heads/").to_owned())
            })
        });
    Ok(RepoStatus {
        initialized: true,
        state: state_name(repo.state()).into(),
        branch,
        entries,
    })
}

// ───────────────────────── commits ─────────────────────────

fn head_commit(repo: &Repository) -> Option<Commit<'_>> {
    repo.head().ok().and_then(|h| h.peel_to_commit().ok())
}

fn stage_all(repo: &Repository) -> Result<()> {
    let mut index = repo.index()?;
    index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
    index.update_all(["*"], None)?;
    index.write()?;
    Ok(())
}

fn changed_count(repo: &Repository, old: Option<&Tree>, new: &Tree) -> usize {
    repo.diff_tree_to_tree(old, Some(new), None)
        .map(|d| d.deltas().len())
        .unwrap_or(0)
}

pub fn checkpoint_message(files: usize) -> String {
    if files == 1 {
        format!("{CHECKPOINT_PREFIX}: 1 arquivo alterado")
    } else {
        format!("{CHECKPOINT_PREFIX}: {files} arquivos alterados")
    }
}

/// Faz stage de tudo e commita. `None` se nada mudou (a menos de `allow_empty`).
fn commit_all(
    repo: &Repository,
    id: &Identity,
    message: &str,
    allow_empty: bool,
) -> Result<Option<Oid>> {
    stage_all(repo)?;
    commit_index(repo, id, message, false, allow_empty)
}

fn commit_index(
    repo: &Repository,
    id: &Identity,
    message: &str,
    amend: bool,
    allow_empty: bool,
) -> Result<Option<Oid>> {
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;
    let parent = head_commit(repo);
    let sig = signature(id)?;
    if amend {
        let head = parent.ok_or_else(|| Error::Other("nada para emendar".into()))?;
        let oid = head.amend(
            Some("HEAD"),
            None,
            Some(&sig),
            None,
            Some(message),
            Some(&tree),
        )?;
        return Ok(Some(oid));
    }
    if let Some(p) = &parent {
        if p.tree_id() == tree_oid && !allow_empty {
            return Ok(None);
        }
    } else if tree.is_empty() && !allow_empty {
        return Ok(None);
    }
    let parents: Vec<&Commit> = parent.iter().collect();
    Ok(Some(repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &parents,
    )?))
}

/// Checkpoint: salva tudo o que mudou. Mensagem automática se `message` for `None`.
/// Não faz nada (e falha com `Busy`) se o repositório estiver em merge/conflito.
pub fn checkpoint(root: &Path, id: &Identity, message: Option<&str>) -> Result<Option<CommitInfo>> {
    let repo = open(root)?;
    ensure_idle(&repo)?;
    checkpoint_in(&repo, id, message)
}

fn checkpoint_in(
    repo: &Repository,
    id: &Identity,
    message: Option<&str>,
) -> Result<Option<CommitInfo>> {
    stage_all(repo)?;
    let mut index = repo.index()?;
    let tree = repo.find_tree(index.write_tree()?)?;
    let parent = head_commit(repo);
    let parent_tree = parent.as_ref().and_then(|p| p.tree().ok());
    let files = changed_count(repo, parent_tree.as_ref(), &tree);
    if files == 0 {
        return Ok(None);
    }
    let msg = message
        .map(str::to_owned)
        .unwrap_or_else(|| checkpoint_message(files));
    match commit_index(repo, id, &msg, false, false)? {
        Some(oid) => Ok(Some(commit_info(repo, oid)?)),
        None => Ok(None),
    }
}

/// Commit manual do que está no stage (ou emenda o último commit).
pub fn commit_staged(
    root: &Path,
    id: &Identity,
    message: &str,
    amend: bool,
) -> Result<Option<CommitInfo>> {
    let repo = open(root)?;
    if message.trim().is_empty() {
        return Err(Error::Other(
            "a mensagem do commit não pode ser vazia".into(),
        ));
    }
    match commit_index(&repo, id, message.trim(), amend, false)? {
        Some(oid) => Ok(Some(commit_info(&repo, oid)?)),
        None => Ok(None),
    }
}

pub fn stage(root: &Path, paths: &[String]) -> Result<()> {
    let repo = open(root)?;
    let mut index = repo.index()?;
    for p in paths {
        let rel = safe_rel(p)?;
        if root.join(&rel).exists() {
            index.add_path(&rel)?;
        } else {
            index.remove_path(&rel)?;
        }
    }
    index.write()?;
    Ok(())
}

pub fn unstage(root: &Path, paths: &[String]) -> Result<()> {
    let repo = open(root)?;
    let rels: Vec<PathBuf> = paths.iter().map(|p| safe_rel(p)).collect::<Result<_>>()?;
    match head_commit(&repo) {
        Some(head) => {
            repo.reset_default(Some(head.as_object()), rels.iter())?;
        }
        None => {
            let mut index = repo.index()?;
            for r in &rels {
                let _ = index.remove_path(r);
            }
            index.write()?;
        }
    }
    Ok(())
}

// ───────────────────────── log ─────────────────────────

fn tag_map(repo: &Repository) -> HashMap<Oid, Vec<TagInfo>> {
    let mut map: HashMap<Oid, Vec<TagInfo>> = HashMap::new();
    for t in list_tags_in(repo) {
        if let Ok(oid) = Oid::from_str(&t.target) {
            map.entry(oid).or_default().push(t);
        }
    }
    map
}

fn list_tags_in(repo: &Repository) -> Vec<TagInfo> {
    let mut out = Vec::new();
    let Ok(refs) = repo.references_glob(&format!("refs/tags/{TAG_PREFIX}*")) else {
        return out;
    };
    for r in refs.flatten() {
        let Some(full) = r.name() else { continue };
        let name = full.trim_start_matches("refs/tags/").to_owned();
        let Ok(commit) = r.peel_to_commit() else {
            continue;
        };
        let (label, description, time) =
            match r.peel(ObjectType::Tag).ok().and_then(|o| o.into_tag().ok()) {
                Some(tag) => {
                    let msg = tag.message().unwrap_or("").to_owned();
                    let (first, rest) = msg.split_once('\n').unwrap_or((msg.as_str(), ""));
                    let time = tag.tagger().map(|s| s.when().seconds()).unwrap_or(0);
                    (first.trim().to_owned(), rest.trim().to_owned(), time)
                }
                None => (
                    name.trim_start_matches(TAG_PREFIX).to_owned(),
                    String::new(),
                    commit.time().seconds(),
                ),
            };
        out.push(TagInfo {
            name,
            label,
            description,
            target: commit.id().to_string(),
            time,
        });
    }
    out.sort_by_key(|t| std::cmp::Reverse(t.time));
    out
}

fn commit_info_with(
    repo: &Repository,
    commit: &Commit,
    tags: &HashMap<Oid, Vec<TagInfo>>,
) -> CommitInfo {
    let tree = commit.tree().ok();
    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
    let files = tree
        .as_ref()
        .map(|t| changed_count(repo, parent_tree.as_ref(), t))
        .unwrap_or(0);
    let message = commit.message().unwrap_or("").trim_end().to_owned();
    let summary = message.lines().next().unwrap_or("").to_owned();
    let author = commit.author();
    CommitInfo {
        id: commit.id().to_string(),
        short: commit.id().to_string().chars().take(7).collect(),
        parent: commit.parent_id(0).ok().map(|o| o.to_string()),
        checkpoint: summary.starts_with(CHECKPOINT_PREFIX),
        summary,
        message,
        author: author.name().unwrap_or("").to_owned(),
        email: author.email().unwrap_or("").to_owned(),
        time: commit.time().seconds(),
        files_changed: files as u32,
        tags: tags.get(&commit.id()).cloned().unwrap_or_default(),
    }
}

fn commit_info(repo: &Repository, oid: Oid) -> Result<CommitInfo> {
    let commit = repo.find_commit(oid)?;
    Ok(commit_info_with(repo, &commit, &tag_map(repo)))
}

/// Log paginado (mais recentes primeiro) com filtros. Devolve `limit` itens após `skip`
/// que satisfazem os filtros.
pub fn log(root: &Path, q: &LogQuery) -> Result<Vec<CommitInfo>> {
    let repo = open(root)?;
    if head_commit(&repo).is_none() {
        return Ok(Vec::new());
    }
    let limit = if q.limit == 0 { 50 } else { q.limit };
    let tags = tag_map(&repo);
    let author = q
        .author
        .as_ref()
        .map(|a| a.to_lowercase())
        .filter(|a| !a.is_empty());
    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    let mut out = Vec::new();
    let mut matched = 0usize;
    for oid in walk {
        let commit = repo.find_commit(oid?)?;
        if q.named_only && !tags.contains_key(&commit.id()) {
            continue;
        }
        if let Some(a) = &author {
            let sig = commit.author();
            let hay = format!("{} {}", sig.name().unwrap_or(""), sig.email().unwrap_or(""))
                .to_lowercase();
            if !hay.contains(a) {
                continue;
            }
        }
        if let Some(path) = q.path.as_deref().filter(|p| !p.is_empty()) {
            let tree = commit.tree()?;
            let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
            let mut opts = DiffOptions::new();
            opts.pathspec(path);
            let diff =
                repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?;
            if diff.deltas().len() == 0 {
                continue;
            }
        }
        matched += 1;
        if matched <= q.skip {
            continue;
        }
        out.push(commit_info_with(&repo, &commit, &tags));
        if out.len() >= limit {
            break;
        }
    }
    Ok(out)
}

// ───────────────────────── diff ─────────────────────────

fn tree_for<'r>(repo: &'r Repository, rev: Option<&str>) -> Result<Option<Tree<'r>>> {
    match rev {
        None => Ok(None),
        Some(r) => Ok(Some(repo.revparse_single(r)?.peel_to_commit()?.tree()?)),
    }
}

fn make_diff<'r>(repo: &'r Repository, from: Option<&str>, to: Option<&str>) -> Result<Diff<'r>> {
    let old = tree_for(repo, from)?;
    let mut opts = DiffOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);
    match to {
        Some(_) => {
            let new = tree_for(repo, to)?;
            Ok(repo.diff_tree_to_tree(old.as_ref(), new.as_ref(), Some(&mut opts))?)
        }
        None => Ok(repo.diff_tree_to_workdir_with_index(old.as_ref(), Some(&mut opts))?),
    }
}

/// Arquivos alterados entre `from` (`None` = árvore vazia) e `to` (`None` = pasta de trabalho).
pub fn changed_files(root: &Path, from: Option<&str>, to: Option<&str>) -> Result<Vec<FileDiff>> {
    let repo = open(root)?;
    let diff = make_diff(&repo, from, to)?;
    let mut out = Vec::new();
    for i in 0..diff.deltas().len() {
        let Some(delta) = diff.get_delta(i) else {
            continue;
        };
        let Some(path) = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
        else {
            continue;
        };
        let status = match delta.status() {
            Delta::Added | Delta::Untracked => "added",
            Delta::Deleted => "deleted",
            Delta::Renamed => "renamed",
            Delta::Conflicted => "conflicted",
            _ => "modified",
        };
        let (additions, deletions, binary) = match Patch::from_diff(&diff, i)? {
            Some(patch) => {
                let (_, a, d) = patch.line_stats()?;
                (a as u32, d as u32, patch.delta().flags().is_binary())
            }
            None => (0, 0, true),
        };
        out.push(FileDiff {
            path,
            status: status.into(),
            additions,
            deletions,
            binary,
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn blob_at(repo: &Repository, rev: Option<&str>, path: &str) -> Result<Option<Vec<u8>>> {
    let Some(tree) = tree_for(repo, rev)? else {
        return Ok(None);
    };
    let Ok(entry) = tree.get_path(Path::new(path)) else {
        return Ok(None);
    };
    Ok(Some(repo.find_blob(entry.id())?.content().to_vec()))
}

fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8000).any(|b| *b == 0)
}

fn mime_for(path: &str) -> Option<&'static str> {
    let ext = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        _ => return None,
    })
}

/// Conteúdo de um arquivo nas duas pontas do diff (texto, ou base64 para binários).
pub fn file_sides(
    root: &Path,
    from: Option<&str>,
    to: Option<&str>,
    path: &str,
) -> Result<FileSides> {
    let repo = open(root)?;
    let rel = safe_rel(path)?;
    let old = blob_at(&repo, from, path)?;
    let new = match to {
        Some(_) => blob_at(&repo, to, path)?,
        None => fs::read(root.join(&rel)).ok(),
    };
    let binary = old.as_deref().is_some_and(is_binary) || new.as_deref().is_some_and(is_binary);
    let text = |b: &Option<Vec<u8>>| b.as_ref().map(|b| String::from_utf8_lossy(b).into_owned());
    let data = |b: &Option<Vec<u8>>| {
        b.as_ref()
            .filter(|b| b.len() <= MAX_PREVIEW_BYTES)
            .map(|b| base64(b))
    };
    Ok(if binary {
        FileSides {
            path: path.to_owned(),
            binary: true,
            old_text: None,
            new_text: None,
            old_data: data(&old),
            new_data: data(&new),
            mime: mime_for(path).map(str::to_owned),
        }
    } else {
        FileSides {
            path: path.to_owned(),
            binary: false,
            old_text: text(&old),
            new_text: text(&new),
            old_data: None,
            new_data: None,
            mime: None,
        }
    })
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

// ───────────────────────── versões nomeadas ─────────────────────────

fn slugify(label: &str) -> String {
    let mut out = String::new();
    for c in label.trim().chars().flat_map(char::to_lowercase) {
        let c = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_owned();
    if out.is_empty() {
        "versao".into()
    } else {
        out
    }
}

fn tag_message(label: &str, description: &str) -> String {
    let d = description.trim();
    if d.is_empty() {
        format!("{}\n", label.trim())
    } else {
        format!("{}\n\n{d}\n", label.trim())
    }
}

fn create_tag(
    repo: &Repository,
    id: &Identity,
    target: Oid,
    label: &str,
    description: &str,
) -> Result<TagInfo> {
    if label.trim().is_empty() {
        return Err(Error::Other("o nome da versão não pode ser vazio".into()));
    }
    let base = slugify(label);
    let mut name = format!("{TAG_PREFIX}{base}");
    let mut n = 2;
    while repo.find_reference(&format!("refs/tags/{name}")).is_ok() {
        name = format!("{TAG_PREFIX}{base}-{n}");
        n += 1;
    }
    let obj = repo.find_object(target, Some(ObjectType::Commit))?;
    repo.tag(
        &name,
        &obj,
        &signature(id)?,
        &tag_message(label, description),
        false,
    )?;
    list_tags_in(repo)
        .into_iter()
        .find(|t| t.name == name)
        .ok_or_else(|| Error::Other("falha ao criar a versão".into()))
}

/// Nomeia um ponto da timeline (`target` = commit; `None` = HEAD atual).
pub fn tag_create(
    root: &Path,
    id: &Identity,
    target: Option<&str>,
    label: &str,
    description: &str,
) -> Result<TagInfo> {
    let repo = open(root)?;
    let oid = match target {
        Some(t) => repo.revparse_single(t)?.peel_to_commit()?.id(),
        None => head_commit(&repo)
            .ok_or_else(|| Error::Other("ainda não há versões para nomear".into()))?
            .id(),
    };
    create_tag(&repo, id, oid, label, description)
}

/// "Nomear versão atual": cria checkpoint se houver mudanças e marca o resultado.
pub fn tag_current(root: &Path, id: &Identity, label: &str, description: &str) -> Result<TagInfo> {
    let repo = open(root)?;
    ensure_idle(&repo)?;
    checkpoint_in(&repo, id, None)?;
    let head = head_commit(&repo)
        .ok_or_else(|| Error::Other("ainda não há versões para nomear".into()))?;
    create_tag(&repo, id, head.id(), label, description)
}

pub fn tag_list(root: &Path) -> Result<Vec<TagInfo>> {
    Ok(list_tags_in(&open(root)?))
}

pub fn tag_rename(
    root: &Path,
    id: &Identity,
    name: &str,
    label: &str,
    description: Option<&str>,
) -> Result<TagInfo> {
    let repo = open(root)?;
    let old = list_tags_in(&repo)
        .into_iter()
        .find(|t| t.name == name)
        .ok_or_else(|| Error::Other("versão não encontrada".into()))?;
    let target = Oid::from_str(&old.target)?;
    repo.tag_delete(name)?;
    create_tag(
        &repo,
        id,
        target,
        label,
        description.unwrap_or(&old.description),
    )
}

pub fn tag_delete(root: &Path, name: &str) -> Result<()> {
    if !name.starts_with(TAG_PREFIX) {
        return Err(Error::Other(
            "só versões do TexLocal podem ser removidas".into(),
        ));
    }
    open(root)?.tag_delete(name)?;
    Ok(())
}

// ───────────────────────── restaurar ─────────────────────────

fn safe_rel(p: &str) -> Result<PathBuf> {
    let path = Path::new(p);
    if p.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || path
            .components()
            .next()
            .is_some_and(|c| c.as_os_str() == ".git")
    {
        return Err(Error::BadPath(format!("caminho inválido: {p}")));
    }
    Ok(path.to_path_buf())
}

fn tree_files(tree: &Tree) -> Vec<(String, Oid)> {
    let mut out = Vec::new();
    let _ = tree.walk(TreeWalkMode::PreOrder, |dir, entry| {
        if entry.kind() == Some(ObjectType::Blob) {
            if let Some(name) = entry.name() {
                out.push((format!("{dir}{name}"), entry.id()));
            }
        }
        TreeWalkResult::Ok
    });
    out
}

fn restore_label(repo: &Repository, target: &Commit) -> String {
    let tags = tag_map(repo);
    tags.get(&target.id())
        .and_then(|t| t.first())
        .map(|t| t.label.clone())
        .unwrap_or_else(|| target.id().to_string().chars().take(7).collect())
}

/// Restaura arquivos (ou o projeto inteiro, `paths = None`) para o estado de `target`.
/// Cria um checkpoint antes e o commit "Restaurado de <versão>" depois; não reescreve histórico.
pub fn restore(
    root: &Path,
    id: &Identity,
    target: &str,
    paths: Option<&[String]>,
) -> Result<RestoreResult> {
    let repo = open(root)?;
    ensure_idle(&repo)?;
    let target_commit = repo.revparse_single(target)?.peel_to_commit()?;
    let target_tree = target_commit.tree()?;
    let label = restore_label(&repo, &target_commit);
    let checkpoint = checkpoint_in(&repo, id, None)?;

    let target_files: HashMap<String, Oid> = tree_files(&target_tree).into_iter().collect();
    let wanted: Vec<String> = match paths {
        Some(p) => {
            for x in p {
                safe_rel(x)?;
            }
            p.to_vec()
        }
        None => {
            let mut all: Vec<String> = target_files.keys().cloned().collect();
            if let Some(head) = head_commit(&repo) {
                for (p, _) in tree_files(&head.tree()?) {
                    if !target_files.contains_key(&p) {
                        all.push(p);
                    }
                }
            }
            all
        }
    };

    let mut touched = Vec::new();
    for p in wanted {
        let abs = root.join(safe_rel(&p)?);
        match target_files.get(&p) {
            Some(oid) => {
                let blob = repo.find_blob(*oid)?;
                if fs::read(&abs).ok().as_deref() == Some(blob.content()) {
                    continue;
                }
                if let Some(dir) = abs.parent() {
                    fs::create_dir_all(dir)?;
                }
                fs::write(&abs, blob.content())?;
                touched.push(p);
            }
            None => {
                if abs.is_file() {
                    fs::remove_file(&abs)?;
                    touched.push(p);
                }
            }
        }
    }

    let message = match paths {
        Some(p) if p.len() == 1 => format!("Restaurado de {label}: {}", p[0]),
        Some(p) => format!("Restaurado de {label}: {} arquivos", p.len()),
        None => format!("Restaurado de {label}"),
    };
    stage_all(&repo)?;
    let commit = commit_index(&repo, id, &message, false, false)?
        .map(|oid| commit_info(&repo, oid))
        .transpose()?;
    Ok(RestoreResult {
        checkpoint,
        commit,
        touched,
    })
}

/// Restaura um trecho: grava `text` em `path` (calculado pela UI a partir do hunk escolhido),
/// com checkpoint antes e commit "Restaurado trecho de ..." depois.
pub fn restore_text(
    root: &Path,
    id: &Identity,
    source: &str,
    path: &str,
    text: &str,
) -> Result<RestoreResult> {
    let repo = open(root)?;
    ensure_idle(&repo)?;
    let source_commit = repo.revparse_single(source)?.peel_to_commit()?;
    let label = restore_label(&repo, &source_commit);
    let checkpoint = checkpoint_in(&repo, id, None)?;
    let abs = root.join(safe_rel(path)?);
    if let Some(dir) = abs.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&abs, text)?;
    stage_all(&repo)?;
    let commit = commit_index(
        &repo,
        id,
        &format!("Restaurado trecho de {label}: {path}"),
        false,
        false,
    )?
    .map(|oid| commit_info(&repo, oid))
    .transpose()?;
    Ok(RestoreResult {
        checkpoint,
        commit,
        touched: vec![path.to_owned()],
    })
}

// ───────────────────────── exportar ─────────────────────────

/// Resolve uma revisão (id, tag, `HEAD`) para o id completo do commit.
pub fn resolve(root: &Path, rev: &str) -> Result<String> {
    Ok(open(root)?
        .revparse_single(rev)?
        .peel_to_commit()?
        .id()
        .to_string())
}

/// Exporta os fontes da versão `rev` (equivalente a `git archive`) para um zip.
pub fn export_zip(root: &Path, rev: &str, dest: &Path) -> Result<usize> {
    let repo = open(root)?;
    let tree = repo.revparse_single(rev)?.peel_to_commit()?.tree()?;
    let file = fs::File::create(dest)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut count = 0;
    for (path, oid) in tree_files(&tree) {
        if path.starts_with(".texlocal/") {
            continue;
        }
        zip.start_file(path, opts)?;
        zip.write_all(repo.find_blob(oid)?.content())?;
        count += 1;
    }
    zip.finish()?;
    Ok(count)
}

/// Extrai a árvore de `rev` para `dest` (worktree temporária para compilar versões antigas).
pub fn export_tree(root: &Path, rev: &str, dest: &Path) -> Result<usize> {
    let repo = open(root)?;
    let tree = repo.revparse_single(rev)?.peel_to_commit()?.tree()?;
    let mut count = 0;
    for (path, oid) in tree_files(&tree) {
        let abs = dest.join(safe_rel(&path)?);
        if let Some(dir) = abs.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(abs, repo.find_blob(oid)?.content())?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> Identity {
        Identity {
            name: "Ana".into(),
            email: "ana@example.com".into(),
        }
    }

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.tex"), "Olá\n").unwrap();
        dir
    }

    #[test]
    fn init_creates_initial_commit_and_files() {
        let dir = project();
        let c = init(dir.path(), &id()).unwrap().unwrap();
        assert_eq!(c.summary, "Versão inicial");
        assert!(dir.path().join(".gitignore").exists());
        let attrs = fs::read_to_string(dir.path().join(".gitattributes")).unwrap();
        assert!(attrs.contains("* text=auto eol=lf"));
        let repo = Repository::open(dir.path()).unwrap();
        assert!(!repo.config().unwrap().get_bool("core.autocrlf").unwrap());
        // idempotente: não recria nada em repositório existente
        assert!(init(dir.path(), &id()).unwrap().is_none());
    }

    #[test]
    fn checkpoint_only_when_changed_and_message_is_generated() {
        let dir = project();
        init(dir.path(), &id()).unwrap();
        assert!(checkpoint(dir.path(), &id(), None).unwrap().is_none());
        fs::write(dir.path().join("main.tex"), "Olá, mundo\n").unwrap();
        fs::write(dir.path().join("b.tex"), "x\n").unwrap();
        let c = checkpoint(dir.path(), &id(), None).unwrap().unwrap();
        assert_eq!(c.summary, "chore(checkpoint): 2 arquivos alterados");
        assert!(c.checkpoint);
        assert_eq!(c.author, "Ana");
    }

    #[test]
    fn build_artifacts_are_ignored() {
        let dir = project();
        init(dir.path(), &id()).unwrap();
        fs::write(dir.path().join("main.aux"), "x").unwrap();
        fs::create_dir_all(dir.path().join(".texlocal/build")).unwrap();
        fs::write(dir.path().join(".texlocal/build/main.pdf"), "x").unwrap();
        assert!(checkpoint(dir.path(), &id(), None).unwrap().is_none());
    }

    #[test]
    fn named_versions_and_filters() {
        let dir = project();
        init(dir.path(), &id()).unwrap();
        fs::write(dir.path().join("main.tex"), "v2\n").unwrap();
        let tag = tag_current(dir.path(), &id(), "Enviado à banca", "versão final").unwrap();
        assert_eq!(tag.name, "texlocal/enviado-a-banca");
        assert_eq!(tag.label, "Enviado à banca");
        assert_eq!(tag.description, "versão final");
        let named = log(
            dir.path(),
            &LogQuery {
                named_only: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(named.len(), 1);
        assert_eq!(named[0].tags[0].label, "Enviado à banca");
        let all = log(dir.path(), &LogQuery::default()).unwrap();
        assert_eq!(all.len(), 2);
        let by_path = log(
            dir.path(),
            &LogQuery {
                path: Some("nada.tex".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(by_path.is_empty());
        let renamed = tag_rename(dir.path(), &id(), &tag.name, "Final", None).unwrap();
        assert_eq!(renamed.label, "Final");
        assert_eq!(renamed.description, "versão final");
        tag_delete(dir.path(), &renamed.name).unwrap();
        assert!(tag_list(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn duplicate_labels_get_unique_refs() {
        let dir = project();
        init(dir.path(), &id()).unwrap();
        let a = tag_create(dir.path(), &id(), None, "v1", "").unwrap();
        let b = tag_create(dir.path(), &id(), None, "v1", "").unwrap();
        assert_ne!(a.name, b.name);
    }

    #[test]
    fn log_paginates() {
        let dir = project();
        init(dir.path(), &id()).unwrap();
        for i in 0..5 {
            fs::write(dir.path().join("main.tex"), format!("{i}\n")).unwrap();
            checkpoint(dir.path(), &id(), None).unwrap();
        }
        let page1 = log(
            dir.path(),
            &LogQuery {
                limit: 4,
                ..Default::default()
            },
        )
        .unwrap();
        let page2 = log(
            dir.path(),
            &LogQuery {
                skip: 4,
                limit: 4,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(page1.len(), 4);
        assert_eq!(page2.len(), 2);
        assert_eq!(page2[1].summary, "Versão inicial");
    }

    #[test]
    fn diff_between_versions_and_workdir() {
        let dir = project();
        let first = init(dir.path(), &id()).unwrap().unwrap();
        fs::write(dir.path().join("main.tex"), "Olá\nnova linha\n").unwrap();
        let second = checkpoint(dir.path(), &id(), None).unwrap().unwrap();
        let files = changed_files(dir.path(), Some(&first.id), Some(&second.id)).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!((files[0].additions, files[0].deletions), (1, 0));
        fs::write(dir.path().join("novo.tex"), "x\n").unwrap();
        let wd = changed_files(dir.path(), Some(&second.id), None).unwrap();
        assert_eq!(wd.len(), 1);
        assert_eq!(wd[0].status, "added");
        let sides = file_sides(dir.path(), Some(&first.id), Some(&second.id), "main.tex").unwrap();
        assert_eq!(sides.old_text.as_deref(), Some("Olá\n"));
        assert_eq!(sides.new_text.as_deref(), Some("Olá\nnova linha\n"));
    }

    #[test]
    fn binary_files_are_flagged_with_preview_data() {
        let dir = project();
        fs::write(dir.path().join("fig.png"), [0u8, 1, 2, 3]).unwrap();
        let first = init(dir.path(), &id()).unwrap().unwrap();
        fs::write(dir.path().join("fig.png"), [0u8, 9, 9]).unwrap();
        checkpoint(dir.path(), &id(), None).unwrap();
        let files = changed_files(dir.path(), Some(&first.id), Some("HEAD")).unwrap();
        assert!(files[0].binary);
        let s = file_sides(dir.path(), Some(&first.id), Some("HEAD"), "fig.png").unwrap();
        assert!(s.binary);
        assert_eq!(s.old_data.as_deref(), Some("AAECAw=="));
        assert_eq!(s.mime.as_deref(), Some("image/png"));
    }

    #[test]
    fn restore_creates_new_commits_and_keeps_history() {
        let dir = project();
        let first = init(dir.path(), &id()).unwrap().unwrap();
        fs::write(dir.path().join("main.tex"), "alterado\n").unwrap();
        checkpoint(dir.path(), &id(), None).unwrap();
        fs::write(dir.path().join("main.tex"), "não salvo\n").unwrap();
        fs::write(dir.path().join("extra.tex"), "x\n").unwrap();
        let r = restore(dir.path(), &id(), &first.id, None).unwrap();
        assert!(r.checkpoint.is_some(), "checkpoint antes de restaurar");
        let c = r.commit.unwrap();
        assert!(c.summary.starts_with("Restaurado de "));
        assert_eq!(
            fs::read_to_string(dir.path().join("main.tex")).unwrap(),
            "Olá\n"
        );
        assert!(!dir.path().join("extra.tex").exists());
        // histórico preservado: inicial + alterado + checkpoint + restauração
        assert_eq!(log(dir.path(), &LogQuery::default()).unwrap().len(), 4);
        // o conteúdo "não salvo" continua recuperável pelo checkpoint
        let prev = r.checkpoint.unwrap();
        let s = file_sides(dir.path(), None, Some(&prev.id), "main.tex").unwrap();
        assert_eq!(s.new_text.as_deref(), Some("não salvo\n"));
    }

    #[test]
    fn restore_single_file_leaves_others() {
        let dir = project();
        fs::write(dir.path().join("b.tex"), "b1\n").unwrap();
        let first = init(dir.path(), &id()).unwrap().unwrap();
        fs::write(dir.path().join("main.tex"), "m2\n").unwrap();
        fs::write(dir.path().join("b.tex"), "b2\n").unwrap();
        checkpoint(dir.path(), &id(), None).unwrap();
        restore(dir.path(), &id(), &first.id, Some(&["main.tex".to_owned()])).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("main.tex")).unwrap(),
            "Olá\n"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("b.tex")).unwrap(),
            "b2\n"
        );
    }

    #[test]
    fn restore_rejects_path_traversal() {
        let dir = project();
        let first = init(dir.path(), &id()).unwrap().unwrap();
        assert!(restore(
            dir.path(),
            &id(),
            &first.id,
            Some(&["../fora.tex".to_owned()])
        )
        .is_err());
        assert!(restore(
            dir.path(),
            &id(),
            &first.id,
            Some(&[".git/config".to_owned()])
        )
        .is_err());
    }

    #[test]
    fn manual_stage_commit_and_amend() {
        let dir = project();
        init(dir.path(), &id()).unwrap();
        fs::write(dir.path().join("a.tex"), "a\n").unwrap();
        fs::write(dir.path().join("b.tex"), "b\n").unwrap();
        stage(dir.path(), &["a.tex".to_owned()]).unwrap();
        let st = status(dir.path()).unwrap();
        assert!(
            st.entries
                .iter()
                .find(|e| e.path == "a.tex")
                .unwrap()
                .staged
        );
        assert!(
            !st.entries
                .iter()
                .find(|e| e.path == "b.tex")
                .unwrap()
                .staged
        );
        let c = commit_staged(dir.path(), &id(), "feat: capítulo A", false)
            .unwrap()
            .unwrap();
        assert_eq!(c.files_changed, 1);
        let a = commit_staged(dir.path(), &id(), "feat: capítulo A (rev)", true)
            .unwrap()
            .unwrap();
        assert_eq!(a.summary, "feat: capítulo A (rev)");
        assert_eq!(log(dir.path(), &LogQuery::default()).unwrap().len(), 2);
        assert!(commit_staged(dir.path(), &id(), " ", false).is_err());
        unstage(dir.path(), &["b.tex".to_owned()]).unwrap();
    }

    #[test]
    fn checkpoint_refuses_during_merge_state() {
        let dir = project();
        init(dir.path(), &id()).unwrap();
        // simula um merge em andamento
        fs::write(
            dir.path().join(".git/MERGE_HEAD"),
            "0000000000000000000000000000000000000000\n",
        )
        .unwrap();
        fs::write(dir.path().join("main.tex"), "x\n").unwrap();
        assert!(matches!(
            checkpoint(dir.path(), &id(), None),
            Err(Error::Busy(_))
        ));
    }

    #[test]
    fn works_with_cli_created_repos() {
        let dir = project();
        let repo = Repository::init(dir.path()).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("main.tex")).unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = Signature::now("Bob", "bob@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "feat: início", &tree, &[])
            .unwrap();
        assert!(init(dir.path(), &id()).unwrap().is_none());
        let l = log(dir.path(), &LogQuery::default()).unwrap();
        assert_eq!(l[0].author, "Bob");
        assert!(!l[0].checkpoint);
    }

    #[test]
    fn export_zip_contains_sources() {
        let dir = project();
        let first = init(dir.path(), &id()).unwrap().unwrap();
        let out = dir.path().join("out.zip");
        let n = export_zip(dir.path(), &first.id, &out).unwrap();
        assert!(n >= 2);
        let mut z = zip::ZipArchive::new(fs::File::open(out).unwrap()).unwrap();
        assert!(z.by_name("main.tex").is_ok());
    }

    #[test]
    fn slugify_handles_accents_and_symbols() {
        assert_eq!(slugify("Enviado à banca!"), "enviado-a-banca");
        assert_eq!(slugify("???"), "versao");
    }
}
