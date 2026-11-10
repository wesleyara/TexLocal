import { describe, expect, it } from 'vitest'
import type { Diagnostic } from './api'
import {
  countDiagnostics,
  DEFAULT_FILTER,
  diagnosticFiles,
  explanationKey,
  filterDiagnostics,
  isCompileRelevant,
  sameFile,
  searchLog,
  sortDiagnostics,
} from './model'

const d = (over: Partial<Diagnostic>): Diagnostic => ({
  severity: 'error',
  category: 'latex',
  file: 'main.tex',
  line: 1,
  message: 'm',
  explanation: null,
  context: null,
  ...over,
})

describe('compile model', () => {
  const list = [
    d({ severity: 'warning', category: 'box', line: 5 }),
    d({ severity: 'warning', category: 'reference', file: 'ch/a.tex', line: 2 }),
    d({ severity: 'error', line: 9 }),
    d({ severity: 'error', file: 'ch/a.tex', line: 3 }),
  ]

  it('avisos de caixa ficam ocultos por padrão', () => {
    expect(filterDiagnostics(list, DEFAULT_FILTER)).toHaveLength(3)
    expect(filterDiagnostics(list, { ...DEFAULT_FILTER, showBoxes: true })).toHaveLength(4)
  })

  it('filtra por tipo e por arquivo', () => {
    expect(filterDiagnostics(list, { ...DEFAULT_FILTER, kind: 'errors' })).toHaveLength(2)
    expect(filterDiagnostics(list, { ...DEFAULT_FILTER, kind: 'warnings' })).toHaveLength(1)
    expect(filterDiagnostics(list, { ...DEFAULT_FILTER, file: 'ch/a.tex' })).toHaveLength(2)
  })

  it('lista arquivos e conta erros/avisos respeitando as caixas', () => {
    expect(diagnosticFiles(list)).toEqual(['ch/a.tex', 'main.tex'])
    expect(countDiagnostics(list)).toEqual({ errors: 2, warnings: 1 })
    expect(countDiagnostics(list, true)).toEqual({ errors: 2, warnings: 2 })
  })

  it('ordena erros antes de avisos, depois por arquivo e linha', () => {
    const sorted = sortDiagnostics(list).map((x) => `${x.severity}:${x.file}:${x.line}`)
    expect(sorted).toEqual([
      'error:ch/a.tex:3',
      'error:main.tex:9',
      'warning:ch/a.tex:2',
      'warning:main.tex:5',
    ])
  })

  it('reconhece arquivos que disparam compilação', () => {
    expect(isCompileRelevant('/p/main.tex')).toBe(true)
    expect(isCompileRelevant('C:\\p\\refs.BIB')).toBe(true)
    expect(isCompileRelevant('/p/notes.md')).toBe(false)
    expect(isCompileRelevant('/p/Makefile')).toBe(false)
  })

  it('chave de explicação', () => {
    expect(explanationKey(d({ explanation: 'missingDollar' }))).toBe(
      'compile.explain.missingDollar',
    )
    expect(explanationKey(d({}))).toBeNull()
  })

  it('compara caminhos relativos do log com os absolutos do editor', () => {
    expect(sameFile('/p', 'ch/a.tex', '/p/ch/a.tex')).toBe(true)
    expect(sameFile('/p/', 'a.tex', '/p/a.tex')).toBe(true)
    expect(sameFile('C:\\p', 'ch/a.tex', 'c:\\P\\ch\\a.tex')).toBe(true)
    expect(sameFile('/p', 'a.tex', '/q/a.tex')).toBe(false)
  })

  it('busca no log sem diferenciar maiúsculas', () => {
    const log = 'Overfull box\nfoo overfull bar OVERFULL\nnada'
    expect(searchLog(log, 'overfull')).toEqual([
      [0, 0],
      [1, 4],
      [1, 17],
    ])
    expect(searchLog(log, '')).toEqual([])
  })
})

describe('resolveFile', () => {
  it('junta ao projeto e preserva caminhos absolutos', async () => {
    const { resolveFile } = await import('./model')
    expect(resolveFile('/p', 'ch/a.tex')).toBe('/p/ch/a.tex')
    expect(resolveFile('/p/', 'a.tex')).toBe('/p/a.tex')
    expect(resolveFile('C:\\p', 'ch/a.tex')).toBe('C:\\p\\ch\\a.tex')
    expect(resolveFile('/p', '/usr/share/x.sty')).toBe('/usr/share/x.sty')
    expect(resolveFile('/p', 'D:/lib/x.sty')).toBe('D:/lib/x.sty')
  })
})
