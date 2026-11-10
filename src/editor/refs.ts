import { linter, type Diagnostic } from '@codemirror/lint'
import { EditorView, keymap } from '@codemirror/view'
import { Prec, type EditorState, type Extension } from '@codemirror/state'
import { labelProblems, type Problem } from '../project/model'
import type { ProjectView } from '../project/api'

export interface LabelHit {
  name: string
  /** `ref`: dentro de \ref{..}; `label`: dentro de \label{..}. */
  kind: 'ref' | 'label'
  from: number
  to: number
}

const REF_RE =
  /\\(?:(label)|(?:eqref|ref|autoref|cref|Cref|pageref|nameref|vref|labelcref|subref)\*?(?:\[[^\]]*\])?)\{([^}]*)\}/g

/** Label sob a posição `pos` (dentro de `\ref{..}`, `\cref{a,b}` ou `\label{..}`). */
export function labelAt(state: EditorState, pos: number): LabelHit | null {
  const line = state.doc.lineAt(pos)
  for (const m of line.text.matchAll(REF_RE)) {
    const argStart = line.from + (m.index ?? 0) + m[0].length - m[2].length - 1
    let offset = argStart
    for (const part of m[2].split(',')) {
      const name = part.trim()
      const from = offset + (part.length - part.trimStart().length)
      const to = from + name.length
      if (name && pos >= from && pos <= to) {
        return { name, kind: m[1] ? 'label' : 'ref', from, to }
      }
      offset += part.length + 1
    }
  }
  return null
}

export interface RefHooks {
  view(): ProjectView
  file(): string
  t(key: string, params: Record<string, string>): string
  onGoto(name: string): void
  onUsages(name: string): void
  onRename(name: string): void
}

function toDiagnostic(state: EditorState, p: Problem, hooks: RefHooks): Diagnostic | null {
  if (p.line >= state.doc.lines) return null
  const line = state.doc.line(p.line + 1)
  const from = Math.min(line.from + p.col, line.to)
  const to = Math.min(line.from + p.endCol, line.to)
  return { from, to, severity: p.severity, message: hooks.t(p.messageKey, p.params) }
}

/** Ir para definição, localizar usos, renomear e diagnósticos de labels. */
export function refsExtension(hooks: RefHooks): Extension {
  const hit = (view: EditorView) => labelAt(view.state, view.state.selection.main.head)
  return [
    linter(
      (view) =>
        labelProblems(hooks.view(), hooks.file())
          .map((p) => toDiagnostic(view.state, p, hooks))
          .filter((d): d is Diagnostic => d !== null),
      { delay: 500 },
    ),
    Prec.high(
      keymap.of([
        {
          key: 'F12',
          run: (view) => {
            const h = hit(view)
            if (!h) return false
            if (h.kind === 'ref') hooks.onGoto(h.name)
            else hooks.onUsages(h.name)
            return true
          },
        },
        {
          key: 'Shift-F12',
          run: (view) => {
            const h = hit(view)
            if (!h) return false
            hooks.onUsages(h.name)
            return true
          },
        },
        {
          key: 'F2',
          run: (view) => {
            const h = hit(view)
            if (!h) return false
            hooks.onRename(h.name)
            return true
          },
        },
      ]),
    ),
    EditorView.domEventHandlers({
      mousedown(event, view) {
        if (!(event.ctrlKey || event.metaKey) || event.button !== 0) return false
        const pos = view.posAtCoords({ x: event.clientX, y: event.clientY })
        if (pos === null) return false
        const h = labelAt(view.state, pos)
        if (!h) return false
        event.preventDefault()
        if (h.kind === 'ref') hooks.onGoto(h.name)
        else hooks.onUsages(h.name)
        return true
      },
    }),
  ]
}
