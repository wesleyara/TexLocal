import type { Diagnostic, DiagnosticCategory } from './api'

/** Extensões cuja alteração justifica recompilar. */
const COMPILE_EXTENSIONS = new Set([
  'tex',
  'bib',
  'sty',
  'cls',
  'bst',
  'ltx',
  'cfg',
  'def',
  'bbx',
  'cbx',
  'lbx',
  'dtx',
])

export function isCompileRelevant(path: string): boolean {
  const dot = path.lastIndexOf('.')
  return dot >= 0 && COMPILE_EXTENSIONS.has(path.slice(dot + 1).toLowerCase())
}

export type KindFilter = 'all' | 'errors' | 'warnings'

export interface ProblemFilter {
  kind: KindFilter
  /** Avisos de overfull/underfull (ocultos por padrão). */
  showBoxes: boolean
  /** Caminho relativo; `null` = todos os arquivos. */
  file: string | null
}

export const DEFAULT_FILTER: ProblemFilter = { kind: 'all', showBoxes: false, file: null }

export function filterDiagnostics(list: Diagnostic[], f: ProblemFilter): Diagnostic[] {
  return list.filter((d) => {
    if (d.category === 'box' && !f.showBoxes) return false
    if (f.kind === 'errors' && d.severity !== 'error') return false
    if (f.kind === 'warnings' && d.severity !== 'warning') return false
    if (f.file !== null && d.file !== f.file) return false
    return true
  })
}

/** Arquivos que aparecem nos diagnósticos, em ordem alfabética. */
export function diagnosticFiles(list: Diagnostic[]): string[] {
  return [...new Set(list.map((d) => d.file).filter((f): f is string => f !== null))].sort()
}

export interface Counts {
  errors: number
  warnings: number
}

/** Contagem para a status bar (avisos de caixa só contam quando visíveis). */
export function countDiagnostics(list: Diagnostic[], showBoxes = false): Counts {
  const counts: Counts = { errors: 0, warnings: 0 }
  for (const d of list) {
    if (d.category === 'box' && !showBoxes) continue
    if (d.severity === 'error') counts.errors++
    else counts.warnings++
  }
  return counts
}

/** Ordena: erros antes de avisos, depois por arquivo e linha. */
export function sortDiagnostics(list: Diagnostic[]): Diagnostic[] {
  const rank = (d: Diagnostic) => (d.severity === 'error' ? 0 : 1)
  return [...list].sort(
    (a, b) =>
      rank(a) - rank(b) ||
      (a.file ?? '').localeCompare(b.file ?? '') ||
      (a.line ?? 0) - (b.line ?? 0),
  )
}

const CATEGORY_ORDER: DiagnosticCategory[] = [
  'latex',
  'reference',
  'package',
  'font',
  'bibtex',
  'biber',
  'box',
]

export const categoryRank = (c: DiagnosticCategory): number => CATEGORY_ORDER.indexOf(c)

/** Chave de tradução da explicação amigável, se houver. */
export function explanationKey(d: Diagnostic): string | null {
  return d.explanation ? `compile.explain.${d.explanation}` : null
}

/** Normaliza para comparar caminhos de diagnósticos com os do editor. */
export function sameFile(root: string, relative: string, absolute: string): boolean {
  const norm = (p: string) => p.replace(/\\/g, '/').replace(/\/+$/, '')
  const joined = `${norm(root)}/${norm(relative)}`
  const a = norm(absolute)
  return /^[A-Za-z]:/.test(a) ? joined.toLowerCase() === a.toLowerCase() : joined === a
}

/** Ocorrências de `query` (sem diferenciar maiúsculas) por linha do log: [linha, coluna]. */
export function searchLog(log: string, query: string): Array<[number, number]> {
  if (!query) return []
  const needle = query.toLowerCase()
  const hits: Array<[number, number]> = []
  log.split('\n').forEach((line, i) => {
    const hay = line.toLowerCase()
    let from = 0
    for (;;) {
      const at = hay.indexOf(needle, from)
      if (at < 0) break
      hits.push([i, at])
      from = at + needle.length
    }
  })
  return hits
}

/** Caminho absoluto de um arquivo citado num diagnóstico (relativo à raiz do projeto). */
export function resolveFile(root: string, file: string): string {
  if (file.startsWith('/') || /^[A-Za-z]:[\\/]/.test(file)) return file
  const sep = root.includes('\\') && !root.includes('/') ? '\\' : '/'
  const base = root.replace(/[\\/]+$/, '')
  return `${base}${sep}${sep === '\\' ? file.replace(/\//g, '\\') : file}`
}
