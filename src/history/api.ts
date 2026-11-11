import { invoke } from '@tauri-apps/api/core'
import { isTauri } from '../tauri'

export interface Identity {
  name: string
  email: string
}

export interface TagInfo {
  name: string
  label: string
  description: string
  target: string
  time: number
}

export interface CommitInfo {
  id: string
  short: string
  parent: string | null
  summary: string
  message: string
  author: string
  email: string
  /** Segundos desde a época. */
  time: number
  filesChanged: number
  checkpoint: boolean
  tags: TagInfo[]
}

export interface StatusEntry {
  path: string
  status: 'added' | 'modified' | 'deleted' | 'renamed' | 'conflicted'
  staged: boolean
  unstaged: boolean
  untracked: boolean
}

export interface RepoStatus {
  initialized: boolean
  state: string
  branch: string | null
  entries: StatusEntry[]
}

export interface LogQuery {
  skip: number
  limit: number
  namedOnly: boolean
  path: string | null
  author: string | null
}

export interface FileDiff {
  path: string
  status: 'added' | 'modified' | 'deleted' | 'renamed' | 'conflicted'
  additions: number
  deletions: number
  binary: boolean
}

export interface FileSides {
  path: string
  binary: boolean
  oldText: string | null
  newText: string | null
  oldData: string | null
  newData: string | null
  mime: string | null
}

export interface RestoreResult {
  checkpoint: CommitInfo | null
  commit: CommitInfo | null
  touched: string[]
}

/** Erro devolvido quando ainda não há nome/e-mail configurados. */
export const IDENTITY_REQUIRED = 'identity-required'

export const emptyStatus = (): RepoStatus => ({
  initialized: false,
  state: 'clean',
  branch: null,
  entries: [],
})

export const isBusyError = (e: unknown): boolean => String(e).startsWith('busy:')
export const isIdentityError = (e: unknown): boolean => String(e).includes(IDENTITY_REQUIRED)

const desktopOnly = () => Promise.reject(new Error('Disponível apenas no aplicativo desktop.'))

function call<T>(cmd: string, args: Record<string, unknown>, fallback?: T): Promise<T> {
  if (!isTauri()) return fallback === undefined ? desktopOnly() : Promise.resolve(fallback)
  return invoke<T>(cmd, args)
}

export const identityGet = () => call<Identity | null>('history_identity_get', {}, null)
export const identitySet = (name: string, email: string) =>
  call<Identity>('history_identity_set', { name, email })

export const historyStatus = (root: string) =>
  call<RepoStatus>('history_status', { root }, emptyStatus())
export const historyInit = (root: string) => call<CommitInfo | null>('history_init', { root })
export const historyCheckpoint = (root: string, message?: string) =>
  call<CommitInfo | null>('history_checkpoint', { root, message: message ?? null })
export const historyCommit = (root: string, message: string, amend: boolean) =>
  call<CommitInfo | null>('history_commit', { root, message, amend })
export const historyStage = (root: string, paths: string[], staged: boolean) =>
  call<void>('history_stage', { root, paths, staged })
export const historyLog = (root: string, query: LogQuery) =>
  call<CommitInfo[]>('history_log', { root, query }, [])
export const historyChanged = (root: string, from: string | null, to: string | null) =>
  call<FileDiff[]>('history_changed', { root, from, to }, [])
export const historySides = (root: string, from: string | null, to: string | null, path: string) =>
  call<FileSides>('history_sides', { root, from, to, path })

export const historyTags = (root: string) => call<TagInfo[]>('history_tags', { root }, [])
export const historyTagCreate = (
  root: string,
  target: string | null,
  label: string,
  description: string,
) => call<TagInfo>('history_tag_create', { root, target, label, description })
export const historyTagRename = (
  root: string,
  name: string,
  label: string,
  description: string | null,
) => call<TagInfo>('history_tag_rename', { root, name, label, description })
export const historyTagDelete = (root: string, name: string) =>
  call<void>('history_tag_delete', { root, name })

export const historyRestore = (root: string, target: string, paths: string[] | null) =>
  call<RestoreResult>('history_restore', { root, target, paths })
export const historyRestoreText = (root: string, source: string, path: string, text: string) =>
  call<RestoreResult>('history_restore_text', { root, source, path, text })

export const historyExportZip = (root: string, rev: string, dest: string) =>
  call<number>('history_export_zip', { root, rev, dest })
export const historyExportPdf = (root: string, rev: string, dest: string, binDir: string | null) =>
  call<void>('history_export_pdf', { root, rev, dest, binDir })

export async function pickSavePath(
  defaultName: string,
  name: string,
  extension: string,
): Promise<string | null> {
  if (!isTauri()) return null
  const { save } = await import('@tauri-apps/plugin-dialog')
  return save({ defaultPath: defaultName, filters: [{ name, extensions: [extension] }] })
}

export async function confirmDialog(message: string, title: string): Promise<boolean> {
  if (isTauri()) {
    const { ask } = await import('@tauri-apps/plugin-dialog')
    return ask(message, { title, kind: 'warning' })
  }
  return window.confirm(message)
}
