import { StateEffect, StateField, type Extension } from '@codemirror/state'
import {
  Decoration,
  EditorView,
  ViewPlugin,
  type DecorationSet,
  type ViewUpdate,
} from '@codemirror/view'
import type { Misspelling } from '../project/api'

const setMisspellings = StateEffect.define<Misspelling[]>()
const recheckEffect = StateEffect.define<null>()

const field = StateField.define<Misspelling[]>({
  create: () => [],
  update(value, tr) {
    for (const e of tr.effects) if (e.is(setMisspellings)) return e.value
    if (!tr.docChanged) return value
    return value
      .map((m) => ({
        ...m,
        from: tr.changes.mapPos(m.from, 1),
        to: tr.changes.mapPos(m.to, -1),
      }))
      .filter((m) => m.to > m.from && !tr.changes.touchesRange(m.from, m.to))
  },
  provide: (f) =>
    EditorView.decorations.from(f, (list): DecorationSet =>
      Decoration.set(
        list.map((m) => Decoration.mark({ class: 'cm-misspelled' }).range(m.from, m.to)),
        true,
      ),
    ),
})

export interface SpellHooks {
  /** Idioma efetivo; `null` desativa o corretor. */
  lang(): string | null
  check(text: string, lang: string): Promise<Misspelling[]>
  onMenu(info: { word: string; from: number; to: number; x: number; y: number }): void
}

export function misspellingAt(list: Misspelling[], pos: number): Misspelling | undefined {
  return list.find((m) => pos >= m.from && pos <= m.to)
}

/** Sublinhado de erros ortográficos e menu de contexto (o menu em si é da UI). */
export function spellExtension(hooks: SpellHooks, delay = 700): Extension {
  const plugin = ViewPlugin.fromClass(
    class {
      timer: ReturnType<typeof setTimeout> | undefined
      constructor(readonly view: EditorView) {
        this.schedule(0)
      }
      update(u: ViewUpdate) {
        if (u.docChanged) this.schedule(delay)
        else if (u.transactions.some((tr) => tr.effects.some((e) => e.is(recheckEffect)))) {
          this.schedule(0)
        }
      }
      schedule(ms: number) {
        clearTimeout(this.timer)
        this.timer = setTimeout(() => void this.run(), ms)
      }
      async run() {
        const lang = hooks.lang()
        const view = this.view
        if (!lang) {
          if (view.state.field(field).length) view.dispatch({ effects: setMisspellings.of([]) })
          return
        }
        const doc = view.state.doc
        try {
          const found = await hooks.check(doc.toString(), lang)
          // descarta o resultado se o texto mudou enquanto verificava
          if (view.state.doc === doc) view.dispatch({ effects: setMisspellings.of(found) })
        } catch {
          // dicionário indisponível: sem sublinhado
        }
      }
      destroy() {
        clearTimeout(this.timer)
      }
    },
  )
  return [
    field,
    plugin,
    EditorView.domEventHandlers({
      contextmenu(event, view) {
        const pos = view.posAtCoords({ x: event.clientX, y: event.clientY })
        if (pos === null) return false
        const m = misspellingAt(view.state.field(field), pos)
        if (!m) return false
        event.preventDefault()
        hooks.onMenu({ word: m.word, from: m.from, to: m.to, x: event.clientX, y: event.clientY })
        return true
      },
    }),
    EditorView.baseTheme({
      '.cm-misspelled': {
        textDecoration: 'underline wavy var(--danger)',
        textDecorationSkipInk: 'none',
      },
    }),
  ]
}

/** Força nova verificação (ex.: após mudar idioma ou adicionar palavra ao dicionário). */
export function recheck(view: EditorView): void {
  view.dispatch({ effects: recheckEffect.of(null) })
}
