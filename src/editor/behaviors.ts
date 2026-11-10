import { foldService } from '@codemirror/language'
import { EditorState, type Text } from '@codemirror/state'
import type { Command } from '@codemirror/view'

// ───────────────────────── \begin / \end ─────────────────────────

interface EnvRef {
  kind: 'begin' | 'end'
  name: string
  /** Intervalo do nome (dentro das chaves). */
  from: number
  to: number
}

const ENV_RE = /\\(begin|end)\{([^}]*)\}/g

/** Se `pos` está no nome de um `\begin{..}`/`\end{..}`, devolve-o. */
export function envAt(doc: Text, pos: number): EnvRef | null {
  const line = doc.lineAt(pos)
  for (const m of line.text.matchAll(ENV_RE)) {
    const from = line.from + (m.index ?? 0) + m[0].indexOf('{') + 1
    const to = from + m[2].length
    if (pos >= from && pos <= to) return { kind: m[1] as 'begin' | 'end', name: m[2], from, to }
  }
  return null
}

/** Encontra o `\end` de um `\begin` (ou o `\begin` de um `\end`) respeitando o aninhamento. */
export function findMatchingEnv(doc: Text, env: EnvRef): EnvRef | null {
  const text = doc.toString()
  const all: EnvRef[] = []
  for (const m of text.matchAll(ENV_RE)) {
    if (m[2] !== env.name) continue
    const from = (m.index ?? 0) + m[0].indexOf('{') + 1
    all.push({ kind: m[1] as 'begin' | 'end', name: m[2], from, to: from + m[2].length })
  }
  const i = all.findIndex((e) => e.from === env.from)
  if (i < 0) return null
  let depth = 0
  if (env.kind === 'begin') {
    for (let j = i + 1; j < all.length; j++) {
      if (all[j].kind === 'begin') depth++
      else if (depth-- === 0) return all[j]
    }
  } else {
    for (let j = i - 1; j >= 0; j--) {
      if (all[j].kind === 'end') depth++
      else if (depth-- === 0) return all[j]
    }
  }
  return null
}

/**
 * Renomear o nome de um `\begin{x}` (ou `\end{x}`) renomeia o par na mesma transação (um único
 * passo de desfazer). Usa `transactionFilter`: extensores de transação não podem alterar o texto.
 */
export const envRenameSync = EditorState.transactionFilter.of((tr) => {
  if (!tr.docChanged) return tr
  let extra: { from: number; to: number; insert: string } | null = null
  tr.changes.iterChanges((fromA, toA, fromB) => {
    if (extra) return
    const old = envAt(tr.startState.doc, fromA)
    if (!old || toA > old.to) return
    const now = envAt(tr.newDoc, fromB)
    if (!now || now.kind !== old.kind || now.name === old.name) return
    const pair = findMatchingEnv(tr.startState.doc, old)
    if (!pair || tr.changes.touchesRange(pair.from, pair.to) !== false) return
    extra = {
      from: tr.changes.mapPos(pair.from, 1),
      to: tr.changes.mapPos(pair.to, -1),
      insert: now.name,
    }
  })
  return extra ? [tr, { changes: extra, sequential: true }] : tr
})

const BEGIN_LINE = /^(\s*).*\\begin\{([^}]+)\}(?:\[[^\]]*\])?(?:\{[^}]*\})*\s*$/

/** Enter no fim de `\begin{x}` sem `\end{x}` correspondente cria o `\end{x}`. */
export const insertEndOnEnter: Command = (view) => {
  const { state } = view
  const sel = state.selection.main
  if (!sel.empty) return false
  const line = state.doc.lineAt(sel.head)
  if (sel.head !== line.to) return false
  const m = BEGIN_LINE.exec(line.text)
  if (!m) return false
  const name = m[2]
  let begins = 0
  let ends = 0
  for (const e of state.doc.toString().matchAll(ENV_RE)) {
    if (e[2] !== name) continue
    if (e[1] === 'begin') begins++
    else ends++
  }
  if (begins <= ends) return false
  const indent = m[1]
  const unit = '  '
  const insert = `\n${indent}${unit}\n${indent}\\end{${name}}`
  view.dispatch({
    changes: { from: sel.head, insert },
    selection: { anchor: sel.head + 1 + indent.length + unit.length },
    userEvent: 'input',
    scrollIntoView: true,
  })
  return true
}

// ───────────────────────── dobra de código ─────────────────────────

const SECTION_LEVELS: Record<string, number> = {
  part: -1,
  chapter: 0,
  section: 1,
  subsection: 2,
  subsubsection: 3,
  paragraph: 4,
  subparagraph: 5,
}
const SECTION_LINE =
  /^\s*\\(part|chapter|section|subsection|subsubsection|paragraph|subparagraph)\*?\s*[[{]/

export const latexFolding = foldService.of((state, lineStart, lineEnd) => {
  const doc = state.doc
  const line = doc.lineAt(lineStart)

  const sec = SECTION_LINE.exec(line.text)
  if (sec) {
    const level = SECTION_LEVELS[sec[1]]
    let last = line.number
    for (let n = line.number + 1; n <= doc.lines; n++) {
      const t = doc.line(n).text
      const next = SECTION_LINE.exec(t)
      if ((next && SECTION_LEVELS[next[1]] <= level) || /\\end\{document\}/.test(t)) break
      if (t.trim() !== '') last = n
    }
    return last > line.number ? { from: lineEnd, to: doc.line(last).to } : null
  }

  const begin = /\\begin\{([^}]+)\}(?!.*\\end\{\1\})/.exec(line.text)
  if (begin) {
    const name = begin[1]
    let depth = 0
    for (let n = line.number + 1; n <= doc.lines; n++) {
      const t = doc.line(n).text
      for (const m of t.matchAll(ENV_RE)) {
        if (m[2] !== name) continue
        if (m[1] === 'begin') depth++
        else if (depth-- === 0) {
          const endLine = doc.line(n)
          return endLine.from - 1 > lineEnd ? { from: lineEnd, to: endLine.from - 1 } : null
        }
      }
    }
  }
  return null
})
