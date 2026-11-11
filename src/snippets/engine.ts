import {
  completionStatus,
  startCompletion,
  type Completion,
  type CompletionSource,
} from '@codemirror/autocomplete'
import {
  Annotation,
  EditorSelection,
  EditorState,
  Prec,
  StateEffect,
  StateField,
  type ChangeSpec,
  type Extension,
  type Transaction,
  type TransactionSpec,
} from '@codemirror/state'
import { EditorView, keymap } from '@codemirror/view'
import { expandBody, emptyContext, type Expansion, type VariableContext } from './expand'
import { applyTransform, parseSnippet, usesVariable } from './parser'

interface Part {
  from: number
  to: number
  transform?: Parameters<typeof applyTransform>[1]
}
interface Stop {
  index: number
  parts: Part[]
  choices?: string[]
}
interface Session {
  stops: Stop[]
  current: number
}

const startSession = StateEffect.define<{ stops: Stop[]; current: number }>()
const gotoStop = StateEffect.define<number>()
const endSession = StateEffect.define<null>()
const mirrorAnnotation = Annotation.define<boolean>()

function mapSession(s: Session, tr: Transaction): Session {
  return {
    current: s.current,
    stops: s.stops.map((st) => ({
      ...st,
      parts: st.parts.map((p) => ({
        ...p,
        from: tr.changes.mapPos(p.from, -1),
        to: tr.changes.mapPos(p.to, 1),
      })),
    })),
  }
}

function inside(s: Session, head: number): boolean {
  return s.stops.some((st) => st.parts.some((p) => head >= p.from && head <= p.to))
}

/** Estado da sessão ativa (`null` fora de um snippet). */
export const snippetField = StateField.define<Session | null>({
  create: () => null,
  update(value, tr) {
    let next = value
    for (const e of tr.effects) {
      if (e.is(startSession)) next = { stops: e.value.stops, current: e.value.current }
      else if (e.is(endSession)) next = null
    }
    if (!next) return null
    if (tr.docChanged && next === value) next = mapSession(next, tr)
    for (const e of tr.effects) if (e.is(gotoStop) && next) next = { ...next, current: e.value }
    if (!next) return null
    const started = tr.effects.some((e) => e.is(startSession) || e.is(gotoStop))
    if (
      !started &&
      (tr.docChanged || tr.selection) &&
      !inside(next, tr.state.selection.main.head)
    ) {
      return null
    }
    return next
  },
})

/** Espelha o que o usuário digita no placeholder atual nas demais ocorrências do mesmo `$n`. */
const mirrorFilter = EditorState.transactionFilter.of(
  (tr): TransactionSpec | readonly TransactionSpec[] => {
    const session = tr.startState.field(snippetField, false)
    if (!session || !tr.docChanged || tr.annotation(mirrorAnnotation)) return tr
    const mapped = mapSession(session, tr)
    const stop = mapped.stops[mapped.current]
    if (!stop || stop.parts.length < 2) return tr
    const primary = stop.parts[0]
    const doc = tr.newDoc
    if (primary.to > doc.length || primary.from > primary.to) return tr
    const text = doc.sliceString(primary.from, primary.to)
    const changes: ChangeSpec[] = []
    for (const m of stop.parts.slice(1)) {
      const value = m.transform ? applyTransform(text, m.transform) : text
      if (doc.sliceString(m.from, m.to) !== value)
        changes.push({ from: m.from, to: m.to, insert: value })
    }
    if (!changes.length) return tr
    return [tr, { changes, sequential: true, annotations: mirrorAnnotation.of(true) }]
  },
)

function selectStop(view: EditorView, index: number) {
  const s = view.state.field(snippetField, false)
  if (!s) return
  const stop = s.stops[index]
  const p = stop.parts[0]
  const last = index === s.stops.length - 1
  view.dispatch({
    selection: EditorSelection.range(p.from, p.to),
    effects: last ? [gotoStop.of(index), endSession.of(null)] : gotoStop.of(index),
    scrollIntoView: true,
    userEvent: 'input.snippet',
  })
  if (!last && stop.choices && stop.choices.length > 1) startCompletion(view)
}

export function snippetActive(state: EditorState): boolean {
  return state.field(snippetField, false) != null
}

export function nextStop(view: EditorView): boolean {
  const s = view.state.field(snippetField, false)
  if (!s) return false
  selectStop(view, Math.min(s.current + 1, s.stops.length - 1))
  return true
}

export function prevStop(view: EditorView): boolean {
  const s = view.state.field(snippetField, false)
  if (!s) return false
  if (s.current === 0) return true
  selectStop(view, s.current - 1)
  return true
}

export function exitSnippet(view: EditorView): boolean {
  if (!snippetActive(view.state)) return false
  // Com a lista de sugestões aberta, o Esc fecha só a lista.
  if (completionStatus(view.state)) return false
  view.dispatch({ effects: endSession.of(null) })
  return true
}

/** Fonte de autocomplete das opções de `${1|a,b,c|}` no tab stop atual. */
export const snippetChoiceSource: CompletionSource = (context) => {
  const s = context.state.field(snippetField, false)
  const stop = s?.stops[s.current]
  if (!stop?.choices || stop.choices.length < 2) return null
  const p = stop.parts[0]
  const head = context.state.selection.main.head
  if (head < p.from || head > p.to) return null
  const options: Completion[] = stop.choices.map((c) => ({ label: c, type: 'enum' }))
  return { from: p.from, to: p.to, options, filter: false }
}

export function snippetEngine(): Extension {
  return [
    snippetField,
    mirrorFilter,
    Prec.highest(
      keymap.of([
        { key: 'Tab', run: nextStop },
        { key: 'Shift-Tab', run: prevStop },
        { key: 'Escape', run: exitSnippet },
      ]),
    ),
  ]
}

/** Contexto de variáveis a partir do editor. */
export function variableContext(
  view: EditorView,
  extra: Partial<VariableContext> = {},
): VariableContext {
  const sel = view.state.selection.main
  const line = view.state.doc.lineAt(sel.head)
  const word = view.state.wordAt(sel.head)
  return {
    ...emptyContext(),
    selectedText: view.state.sliceDoc(sel.from, sel.to),
    currentLine: line.text,
    currentWord: word ? view.state.sliceDoc(word.from, word.to) : '',
    lineIndex: line.number - 1,
    ...extra,
  }
}

/** Insere o resultado já expandido em `[from, to)`, abrindo a sessão de tab stops. */
export function applyExpansion(view: EditorView, from: number, to: number, ex: Expansion) {
  const stops: Stop[] = ex.stops.map((s) => ({
    index: s.index,
    choices: s.choices,
    parts: s.parts.map((p) => ({ from: from + p.from, to: from + p.to, transform: p.transform })),
  }))
  const interactive = stops.length > 1 || stops[0].parts.length > 0
  const first = stops[0]
  const spec: TransactionSpec = {
    changes: { from, to, insert: ex.text },
    userEvent: 'input.snippet',
    scrollIntoView: true,
  }
  if (stops.length > 1) {
    const p = first.parts[0]
    view.dispatch({
      ...spec,
      selection: EditorSelection.range(p.from, p.to),
      effects: startSession.of({ stops, current: 0 }),
    })
    if (first.choices && first.choices.length > 1) startCompletion(view)
  } else {
    // Só `$0`: o cursor vai para lá e não há sessão.
    const p = first.parts[0]
    view.dispatch({ ...spec, selection: EditorSelection.cursor(p.to) })
  }
  return interactive
}

/** Expande `body` na posição da seleção (ou em `[from, to)`). */
export async function insertSnippet(
  view: EditorView,
  body: string,
  opts: { from?: number; to?: number; ctx?: Partial<VariableContext> } = {},
): Promise<void> {
  const sel = view.state.selection.main
  const from = opts.from ?? sel.from
  const to = opts.to ?? sel.to
  const nodes = parseSnippet(body.replace(/\t/g, '  '))
  const extra: Partial<VariableContext> = { ...opts.ctx }
  if (usesVariable(nodes, 'CLIPBOARD') && extra.clipboard === undefined) {
    try {
      extra.clipboard = await navigator.clipboard.readText()
    } catch {
      extra.clipboard = ''
    }
  }
  // O texto selecionado é substituído pelo snippet; `$TM_SELECTED_TEXT` o reaproveita.
  const ctx = variableContext(view, {
    ...extra,
    selectedText: view.state.sliceDoc(from, to),
  })
  const indent = /^\s*/.exec(view.state.doc.lineAt(from).text)?.[0] ?? ''
  const ex = expandBody(nodes, ctx, indent)
  applyExpansion(view, from, to, ex)
}
