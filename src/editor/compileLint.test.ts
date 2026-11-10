import { EditorState } from '@codemirror/state'
import { describe, expect, it } from 'vitest'
import type { Diagnostic } from '../compile/api'
import { toCmDiagnostics, type CompileLintHooks } from './compileLint'

const diag = (over: Partial<Diagnostic>): Diagnostic => ({
  severity: 'error',
  category: 'latex',
  file: 'main.tex',
  line: 2,
  message: 'Undefined control sequence.',
  explanation: 'undefinedControlSequence',
  context: null,
  ...over,
})

const hooks = (list: Diagnostic[], boxes = false): CompileLintHooks => ({
  diagnostics: () => list,
  showBoxes: () => boxes,
  t: (k) => `T(${k})`,
  has: (k) => k === 'compile.explain.undefinedControlSequence',
})

describe('compileLint', () => {
  const state = EditorState.create({ doc: 'a\n  \\foo bar\nlast' })

  it('sublinha a linha do erro, sem a indentação, com a explicação amigável', () => {
    const [d] = toCmDiagnostics(state, hooks([diag({})]))
    expect(state.sliceDoc(d.from, d.to)).toBe('\\foo bar')
    expect(d.severity).toBe('error')
    expect(d.message).toBe(
      'T(compile.explain.undefinedControlSequence)\nUndefined control sequence.',
    )
  })

  it('usa a mensagem original quando não há explicação traduzida', () => {
    const [d] = toCmDiagnostics(state, hooks([diag({ explanation: 'desconhecida' })]))
    expect(d.message).toBe('Undefined control sequence.')
  })

  it('ignora linhas inexistentes e sem linha; oculta caixas por padrão', () => {
    const list = [
      diag({ line: 99 }),
      diag({ line: null }),
      diag({ line: 1, category: 'box', severity: 'warning', explanation: null }),
    ]
    expect(toCmDiagnostics(state, hooks(list))).toHaveLength(0)
    expect(toCmDiagnostics(state, hooks(list, true))).toHaveLength(1)
  })

  it('marca avisos de bibliografia com a origem BibTeX', () => {
    const [d] = toCmDiagnostics(
      state,
      hooks([diag({ category: 'biber', severity: 'warning', explanation: null })]),
    )
    expect(d.source).toBe('BibTeX')
    expect(d.severity).toBe('warning')
  })
})
