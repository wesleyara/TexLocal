import {
  autocompletion,
  closeBracketsKeymap,
  completionKeymap,
  type CompletionSource,
} from '@codemirror/autocomplete'
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
import {
  bracketMatching,
  defaultHighlightStyle,
  foldGutter,
  foldKeymap,
  HighlightStyle,
  indentOnInput,
  syntaxHighlighting,
} from '@codemirror/language'
import { lintKeymap } from '@codemirror/lint'
import { highlightSelectionMatches, search, searchKeymap } from '@codemirror/search'
import { Compartment, EditorState, Prec, type Extension } from '@codemirror/state'
import {
  crosshairCursor,
  drawSelection,
  dropCursor,
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
  rectangularSelection,
} from '@codemirror/view'
import { tags as t } from '@lezer/highlight'
import { insertEndOnEnter, latexFolding, envRenameSync } from './behaviors'
import { snippetEngine } from '../snippets/engine'
import { languageFor, languageIdFor } from './languages'

export const wrapCompartment = new Compartment()

const highlight = HighlightStyle.define([
  { tag: t.keyword, color: 'var(--syn-keyword)' },
  { tag: t.comment, color: 'var(--syn-comment)', fontStyle: 'italic' },
  { tag: t.tagName, color: 'var(--syn-env)', fontWeight: '600' },
  { tag: t.bracket, color: 'var(--syn-bracket)' },
  { tag: t.atom, color: 'var(--syn-math)' },
  { tag: t.number, color: 'var(--syn-math)' },
  { tag: t.operator, color: 'var(--syn-bracket)' },
  { tag: t.string, color: 'var(--syn-string)' },
  { tag: t.meta, color: 'var(--syn-bracket)' },
  { tag: t.heading, color: 'var(--syn-env)', fontWeight: '700' },
  { tag: t.strong, fontWeight: '700' },
  { tag: t.link, color: 'var(--accent)', textDecoration: 'underline' },
  { tag: t.variableName, color: 'var(--syn-math)' },
])

const theme = EditorView.theme({
  '&': { height: '100%', color: 'var(--text)', backgroundColor: 'var(--bg)', fontSize: '13.5px' },
  '.cm-scroller': { fontFamily: 'var(--font-mono)', lineHeight: '1.55' },
  '.cm-content': { caretColor: 'var(--text)' },
  '.cm-gutters': {
    backgroundColor: 'var(--bg-panel)',
    color: 'var(--text-muted)',
    border: 'none',
  },
  '.cm-activeLine': { backgroundColor: 'color-mix(in srgb, var(--bg-hover) 55%, transparent)' },
  '.cm-activeLineGutter': { backgroundColor: 'var(--bg-hover)', color: 'var(--text)' },
  '&.cm-focused': { outline: 'none' },
  '&.cm-focused .cm-cursor': { borderLeftColor: 'var(--text)' },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection': {
    backgroundColor: 'color-mix(in srgb, var(--accent) 35%, transparent)',
  },
  '.cm-tooltip': {
    backgroundColor: 'var(--bg-elevated)',
    border: '1px solid var(--border)',
    borderRadius: '8px',
    boxShadow: '0 8px 24px rgb(0 0 0 / 28%), 0 1px 3px rgb(0 0 0 / 20%)',
    color: 'var(--text)',
    overflow: 'hidden',
  },
  '.cm-tooltip.cm-tooltip-autocomplete': { padding: '4px', minWidth: '260px' },
  '.cm-tooltip-autocomplete > ul': {
    fontFamily: 'var(--font-mono)',
    fontSize: '12.5px',
    maxHeight: '264px',
    scrollbarWidth: 'thin',
    scrollbarColor: 'var(--border) transparent',
  },
  '.cm-tooltip-autocomplete > ul > li': {
    display: 'flex',
    alignItems: 'center',
    padding: '3px 8px 3px 6px',
    borderRadius: '5px',
    lineHeight: '1.55',
    transition: 'background-color 80ms',
  },
  '.cm-tooltip-autocomplete > ul > li:hover': { backgroundColor: 'var(--bg-hover)' },
  '.cm-tooltip-autocomplete ul li[aria-selected]': {
    backgroundColor: 'color-mix(in srgb, var(--accent) 24%, transparent)',
    color: 'var(--text)',
    boxShadow: 'inset 2px 0 0 var(--accent)',
  },
  '.cm-completionLabel': { flex: 'none' },
  '.cm-completionMatchedText': {
    textDecoration: 'none',
    fontWeight: '700',
    color: 'var(--accent)',
  },
  '.cm-completionDetail': {
    marginLeft: 'auto',
    paddingLeft: '18px',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
    fontFamily: 'var(--font-ui)',
    fontSize: '11.5px',
    fontStyle: 'normal',
    color: 'var(--text-muted)',
  },
  '.cm-completionIcon': {
    flex: 'none',
    width: '20px',
    height: '20px',
    marginRight: '8px',
    padding: '0',
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: '12px',
    opacity: '1',
    borderRadius: '5px',
    backgroundColor: 'color-mix(in srgb, var(--text-muted) 16%, transparent)',
    color: 'var(--text-muted)',
  },
  '.cm-completionIcon-function, .cm-completionIcon-method': {
    color: 'var(--accent)',
    backgroundColor: 'color-mix(in srgb, var(--accent) 16%, transparent)',
  },
  '.cm-completionIcon-class': {
    color: 'var(--syn-env)',
    backgroundColor: 'color-mix(in srgb, var(--syn-env) 16%, transparent)',
  },
  '.cm-completionIcon-constant, .cm-completionIcon-variable': {
    color: 'var(--syn-math)',
    backgroundColor: 'color-mix(in srgb, var(--syn-math) 16%, transparent)',
  },
  '.cm-completionIcon-keyword, .cm-completionIcon-namespace': {
    color: 'var(--syn-keyword)',
    backgroundColor: 'color-mix(in srgb, var(--syn-keyword) 16%, transparent)',
  },
  '.cm-completionIcon-snippet, .cm-completionIcon-enum': {
    color: 'var(--syn-string)',
    backgroundColor: 'color-mix(in srgb, var(--syn-string) 16%, transparent)',
  },
  '.cm-completionInfo': {
    padding: '8px 12px',
    fontFamily: 'var(--font-ui)',
    fontSize: '12px',
    color: 'var(--text-muted)',
    borderRadius: '8px',
  },
  '.cm-completionIcon-snippet:after': { content: "'✂'" },
  '.cm-snippet-preview': { margin: 0, fontFamily: 'var(--font-mono)', whiteSpace: 'pre' },
  '.cm-panels': { backgroundColor: 'var(--bg-panel)', color: 'var(--text)' },
  '.cm-panels-top': { borderBottom: '1px solid var(--border)' },
  '.cm-searchMatch': { backgroundColor: 'color-mix(in srgb, var(--accent) 30%, transparent)' },
  '.cm-searchMatch-selected': {
    backgroundColor: 'color-mix(in srgb, var(--accent) 60%, transparent)',
  },
  '.cm-textfield': {
    backgroundColor: 'var(--bg)',
    color: 'var(--text)',
    border: '1px solid var(--border)',
  },
  '.cm-button': {
    backgroundImage: 'none',
    backgroundColor: 'var(--bg-hover)',
    color: 'var(--text)',
  },
})

const PHRASES_PT: Record<string, string> = {
  Find: 'Buscar',
  Replace: 'Substituir',
  next: 'próximo',
  previous: 'anterior',
  all: 'todos',
  'match case': 'diferenciar maiúsculas',
  regexp: 'regex',
  'by word': 'palavra inteira',
  replace: 'substituir',
  'replace all': 'substituir todos',
  close: 'fechar',
  'Go to line': 'Ir para a linha',
  go: 'ir',
  'Folded lines': 'Linhas dobradas',
  'Unfolded lines': 'Linhas desdobradas',
  to: 'até',
  'folded code': 'código dobrado',
  unfold: 'desdobrar',
  'Fold line': 'Dobrar linha',
  'Unfold line': 'Desdobrar linha',
  'Control character': 'Caractere de controle',
  'Selection deleted': 'Seleção removida',
  'current match': 'ocorrência atual',
  'on line': 'na linha',
  'replaced match on line $': 'ocorrência substituída na linha $',
  'replaced $ matches': '$ ocorrências substituídas',
  Diagnostics: 'Diagnósticos',
  'No diagnostics': 'Sem diagnósticos',
}

export interface EditorPrefs {
  wrap: boolean
  locale: string
}

export interface SetupOptions {
  path: string
  prefs: EditorPrefs
  completionSources: CompletionSource[]
  /** Extensões que dependem do app (refs, ortografia, listeners). */
  extra: Extension[]
}

/** Extensões de um buffer. O estado completo (undo, seleção) é guardado por arquivo. */
export function editorExtensions(opts: SetupOptions): Extension[] {
  const isLatex = languageIdFor(opts.path) === 'latex'
  return [
    lineNumbers(),
    highlightActiveLineGutter(),
    foldGutter(),
    history(),
    drawSelection(),
    dropCursor(),
    EditorState.allowMultipleSelections.of(true),
    // Alt+clique adiciona cursor; Ctrl/Cmd+clique fica para "ir para definição".
    EditorView.clickAddsSelectionRange.of((e) => e.altKey),
    indentOnInput(),
    bracketMatching(),
    rectangularSelection(),
    crosshairCursor(),
    highlightActiveLine(),
    highlightSelectionMatches(),
    search({ top: true }),
    syntaxHighlighting(highlight),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    EditorState.phrases.of(opts.prefs.locale.startsWith('pt') ? PHRASES_PT : {}),
    theme,
    wrapCompartment.of(opts.prefs.wrap ? EditorView.lineWrapping : []),
    languageFor(opts.path),
    isLatex ? [latexFolding, envRenameSync] : [],
    autocompletion({
      override: opts.completionSources.length ? opts.completionSources : undefined,
      activateOnTyping: true,
    }),
    snippetEngine(),
    Prec.high(keymap.of(isLatex ? [{ key: 'Enter', run: insertEndOnEnter }] : [])),
    keymap.of([
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...searchKeymap,
      ...historyKeymap,
      ...foldKeymap,
      ...completionKeymap,
      ...lintKeymap,
      indentWithTab,
    ]),
    ...opts.extra,
  ]
}

export function wrapEffect(on: boolean) {
  return wrapCompartment.reconfigure(on ? EditorView.lineWrapping : [])
}
