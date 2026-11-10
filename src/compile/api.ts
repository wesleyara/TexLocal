import { invoke } from '@tauri-apps/api/core'
import { isTauri } from '../tauri'

export type CompileStatus = 'success' | 'failed' | 'cancelled' | 'timedOut'
export type Severity = 'error' | 'warning'
export type DiagnosticCategory =
  'latex' | 'reference' | 'box' | 'package' | 'font' | 'bibtex' | 'biber'

export interface Diagnostic {
  severity: Severity
  category: DiagnosticCategory
  /** Relativo à raiz do projeto (com `/`), quando o arquivo pertence a ele. */
  file: string | null
  /** 1-based. */
  line: number | null
  message: string
  /** Id da explicação amigável (`compile.explain.<id>`). */
  explanation: string | null
  context: string | null
}

export interface MissingFile {
  name: string
  kind: string
}

export interface CompileOutcome {
  status: CompileStatus
  pdf: string | null
  pdfUpdated: boolean
  synctex: string | null
  log: string
  output: string
  diagnostics: Diagnostic[]
  missingFiles: MissingFile[]
  durationMs: number
}

export interface CompileError {
  kind: 'latexmkNotFound' | 'mainNotFound' | 'manifest' | 'busy' | 'io' | 'unavailable'
  i18nKey: string
  details: string
}

export interface SyncRect {
  x: number
  y: number
  w: number
  h: number
}

export interface ForwardResult {
  page: number
  rects: SyncRect[]
}

export interface SourcePosition {
  file: string
  line: number
}

export type Engine = 'pdflatex' | 'xelatex' | 'lualatex'

/** Espelho do `texlocal.toml` (nomes de campo iguais aos do TOML). */
export interface Manifest {
  project: { main: string }
  tex: { engine: Engine; texlive?: number; packages: string[]; bibliography?: string }
  compile: {
    auto: boolean
    shell_escape: boolean
    outdir: string
    timeout_secs: number
    debounce_ms: number
  }
  editor?: Record<string, unknown>
}

export function toCompileError(e: unknown): CompileError {
  if (typeof e === 'object' && e !== null && 'i18nKey' in e) return e as CompileError
  return { kind: 'io', i18nKey: 'compile.error.io', details: String(e) }
}

const unavailable = (): never => {
  throw { kind: 'unavailable', i18nKey: 'compile.error.unavailable', details: '' }
}

export async function compileStart(dir: string, binDir?: string): Promise<CompileOutcome> {
  if (!isTauri()) return unavailable()
  return invoke<CompileOutcome>('compile_start', { dir, binDir: binDir ?? null })
}

export async function compileCancel(dir: string): Promise<void> {
  if (isTauri()) await invoke('compile_cancel', { dir })
}

/** Bytes do PDF. No navegador (desenvolvimento) aceita uma URL. */
export async function pdfRead(path: string): Promise<ArrayBuffer> {
  if (!isTauri()) {
    const res = await fetch(path)
    return res.arrayBuffer()
  }
  return invoke<ArrayBuffer>('pdf_read', { path })
}

export async function synctexForward(
  dir: string,
  file: string,
  line: number,
): Promise<ForwardResult | null> {
  if (!isTauri()) return null
  return invoke<ForwardResult | null>('synctex_forward', { dir, file, line })
}

export async function synctexInverse(
  dir: string,
  page: number,
  x: number,
  y: number,
): Promise<SourcePosition | null> {
  if (!isTauri()) return null
  return invoke<SourcePosition | null>('synctex_inverse', { dir, page, x, y })
}

export async function getManifest(dir: string): Promise<Manifest | null> {
  if (!isTauri()) return null
  return invoke<Manifest>('project_manifest', { dir })
}

export async function saveManifest(dir: string, manifest: Manifest): Promise<void> {
  if (isTauri()) await invoke('save_project_manifest', { dir, manifest })
}
