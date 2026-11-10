import { linter, lintGutter, type Diagnostic as CmDiagnostic } from '@codemirror/lint'
import type { EditorState, Extension } from '@codemirror/state'
import type { Diagnostic } from '../compile/api'
import { explanationKey } from '../compile/model'

export interface CompileLintHooks {
  /** Diagnósticos da compilação que pertencem ao arquivo deste buffer. */
  diagnostics: () => Diagnostic[]
  /** Mostrar avisos de caixa (overfull/underfull). */
  showBoxes: () => boolean
  t: (key: string) => string
  /** Existe tradução para a chave? */
  has: (key: string) => boolean
}

/** Converte os diagnósticos da compilação (linhas 1-based) em diagnósticos do CodeMirror. */
export function toCmDiagnostics(state: EditorState, hooks: CompileLintHooks): CmDiagnostic[] {
  const out: CmDiagnostic[] = []
  const boxes = hooks.showBoxes()
  for (const d of hooks.diagnostics()) {
    if (d.line === null || d.line < 1 || d.line > state.doc.lines) continue
    if (d.category === 'box' && !boxes) continue
    const line = state.doc.line(d.line)
    const indent = line.text.length - line.text.trimStart().length
    const from = line.from + indent
    const to = Math.max(line.to, from)
    const key = explanationKey(d)
    const friendly = key && hooks.has(key) ? hooks.t(key) : null
    out.push({
      from,
      to,
      severity: d.severity,
      source: d.category === 'bibtex' || d.category === 'biber' ? 'BibTeX' : 'LaTeX',
      message: friendly ? `${friendly}\n${d.message}` : d.message,
    })
  }
  return out
}

/** Sublinhado, marcador na gutter e tooltip com os erros e avisos da última compilação. */
export function compileLintExtension(hooks: CompileLintHooks): Extension {
  return [linter((view) => toCmDiagnostics(view.state, hooks), { delay: 100 }), lintGutter()]
}
