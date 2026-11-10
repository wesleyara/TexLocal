import { invoke } from '@tauri-apps/api/core'
import { isTauri } from '../tauri'

export type Eol = 'lf' | 'crlf'
export type Encoding = 'utf8' | 'latin1'

export interface FileContent {
  text: string
  eol: Eol
  encoding: Encoding
  bom: boolean
}

export interface Span {
  line: number
  col: number
  endCol: number
}

export interface Section {
  kind: string
  level: number
  title: string
  starred: boolean
  line: number
}
export interface Label {
  name: string
  span: Span
  context: string
}
export interface Reference {
  name: string
  command: string
  span: Span
}
export interface Citation {
  key: string
  span: Span
}
export interface Include {
  kind: 'input' | 'include' | 'subfile' | 'import' | 'graphics' | 'bibliography'
  path: string
  line: number
}
export interface Pkg {
  name: string
  options: string
}
export interface UserCommand {
  name: string
  kind: 'command' | 'operator' | 'environment'
  args: number
  line: number
}
export interface Float {
  kind: string
  caption: string
  label: string | null
  line: number
}

export interface FileIndex {
  documentClass: string | null
  magicRoot: string | null
  sections: Section[]
  labels: Label[]
  references: Reference[]
  citations: Citation[]
  includes: Include[]
  packages: Pkg[]
  commands: UserCommand[]
  floats: Float[]
}

export interface FileEntry {
  path: string
  index: FileIndex
}

export interface ProjectView {
  main: string | null
  files: FileEntry[]
  missing: string[]
}

export interface OpenedProject {
  root: string
  view: ProjectView
  language: string | null
}

export interface SearchQuery {
  pattern: string
  regex: boolean
  caseSensitive: boolean
  wholeWord: boolean
  glob: string
}

export interface SearchMatch {
  line: number
  col: number
  endCol: number
  text: string
  replaced: string | null
}
export interface FileMatches {
  path: string
  matches: SearchMatch[]
}

export interface Counts {
  words: number
  headers: number
  inlineMath: number
  displayMath: number
}
export interface SectionCount {
  file: string
  title: string
  level: number
  words: number
}
export interface ProjectCount {
  total: Counts
  sections: SectionCount[]
}

export interface Misspelling {
  from: number
  to: number
  word: string
}

export const emptyView = (): ProjectView => ({ main: null, files: [], missing: [] })

const unavailable = () => Promise.reject(new Error('disponível apenas no aplicativo desktop'))

/** Executa `invoke` no desktop; no navegador (dev/testes) devolve `fallback` ou rejeita. */
function call<T>(cmd: string, args: Record<string, unknown>, fallback?: () => T): Promise<T> {
  if (isTauri()) return invoke<T>(cmd, args)
  return fallback ? Promise.resolve(fallback()) : (unavailable() as Promise<T>)
}

export const fileRead = (path: string) => call<FileContent>('file_read', { path })

export const fileWrite = (path: string, text: string, eol: Eol, encoding: Encoding, bom: boolean) =>
  call<{ encoding: Encoding }>('file_write', { path, text, eol, encoding, bom })

export const projectFiles = (root: string) => call<string[]>('project_files', { root }, () => [])

export const projectOpen = (root: string) => call<OpenedProject>('project_open', { root })

export const indexUpdate = (root: string, path: string, text: string, setHint: boolean) =>
  call<ProjectView>('index_update', { root, path, text, setHint }, emptyView)

export const indexRemove = (root: string, path: string) =>
  call<ProjectView>('index_remove', { root, path }, emptyView)

export const projectSearch = (root: string, query: SearchQuery, replacement: string | null) =>
  call<FileMatches[]>('project_search', { root, query, replacement }, () => [])

export const projectReplace = (query: SearchQuery, replacement: string, files: string[]) =>
  call<string[]>('project_replace', { query, replacement, files })

const zeroCounts = (): Counts => ({ words: 0, headers: 0, inlineMath: 0, displayMath: 0 })

export const wordCountText = (text: string) => call<Counts>('word_count_text', { text }, zeroCounts)

export const wordCountFiles = (paths: string[]) =>
  call<ProjectCount>('word_count_files', { paths }, () => ({ total: zeroCounts(), sections: [] }))

export const spellConfigure = (root: string | null) =>
  call<string[]>('spell_configure', { root }, () => [])

export const spellCheck = (text: string, lang: string) =>
  call<Misspelling[]>('spell_check', { text, lang }, () => [])

export const spellSuggest = (word: string, lang: string) =>
  call<string[]>('spell_suggest', { word, lang }, () => [])

export const spellDetectLanguage = (text: string) =>
  call<string | null>('spell_detect_language', { text }, () => null)

export const spellAddWord = (word: string, scope: 'global' | 'project', root: string | null) =>
  call<void>('spell_add_word', { word, scope, root })

export async function pickFolder(): Promise<string | null> {
  if (!isTauri()) return null
  const { open } = await import('@tauri-apps/plugin-dialog')
  const picked = await open({ directory: true, multiple: false })
  return typeof picked === 'string' ? picked : null
}

/** Nome do arquivo a partir do caminho (aceita `/` e `\`). */
export const baseName = (path: string): string => path.split(/[\\/]/).pop() ?? path

/** Caminho relativo a `root`, com `/`. Mantém o original se estiver fora da raiz. */
export function relativePath(root: string, path: string): string {
  const norm = (p: string) => p.replace(/\\/g, '/')
  const r = norm(root).replace(/\/$/, '') + '/'
  const p = norm(path)
  return p.startsWith(r) ? p.slice(r.length) : p
}

/** Junta raiz e relativo usando o separador da raiz. */
export function joinPath(root: string, rel: string): string {
  const sep = root.includes('\\') ? '\\' : '/'
  return root.replace(/[\\/]$/, '') + sep + rel.replace(/\//g, sep)
}
