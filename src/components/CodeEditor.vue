<script setup lang="ts">
import { EditorSelection, EditorState } from '@codemirror/state'
import { forceLinting } from '@codemirror/lint'
import { EditorView } from '@codemirror/view'
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { sameFile } from '../compile/model'
import { gotoLabel, registerActiveView, showUsages } from '../editor/actions'
import { buffers } from '../editor/buffers'
import { compileLintExtension } from '../editor/compileLint'
import { latexCompletionSources, type CompletionData } from '../editor/completions'
import { languageIdFor } from '../editor/languages'
import { refsExtension } from '../editor/refs'
import { editorExtensions, wrapEffect } from '../editor/setup'
import { recheck, spellExtension } from '../editor/spellcheck'
import {
  spellAddWord,
  spellCheck,
  spellDetectLanguage,
  spellSuggest,
  wordCountText,
} from '../project/api'
import { useCompileStore } from '../stores/compile'
import { useEditorStore } from '../stores/editor'
import { useProjectStore } from '../stores/project'
import { useRefsStore } from '../stores/refs'

const { t, te, locale } = useI18n()
const editor = useEditorStore()
const project = useProjectStore()
const refs = useRefsStore()
const compile = useCompileStore()

const host = ref<HTMLDivElement | null>(null)
let view: EditorView | null = null
let indexTimer: ReturnType<typeof setTimeout> | undefined
let selTimer: ReturnType<typeof setTimeout> | undefined

interface SpellMenu {
  word: string
  from: number
  to: number
  x: number
  y: number
  suggestions: string[]
}
const spellMenu = ref<SpellMenu | null>(null)

const completionData: CompletionData = {
  packages: () => project.packages,
  documentClass: () => project.docClass,
  userCommands: () => project.commands,
  labels: () => project.labels,
  files: () => project.filesFromMain,
}

function spellLang(): string | null {
  const lang = project.spellLanguage
  return lang && project.dictionaries.includes(lang) ? lang : null
}

async function flushIndex() {
  clearTimeout(indexTimer)
  const path = editor.active
  if (path && view && editor.activeTab) {
    await project.updateIndex(path, view.state.doc.toString(), true)
  }
}

function scheduleIndex(path: string) {
  clearTimeout(indexTimer)
  indexTimer = setTimeout(() => {
    if (languageIdFor(path) !== 'latex') return
    const text = buffers.text(path)
    if (text === undefined) return
    void project.updateIndex(path, text, true)
    if (!project.manifestLanguage) {
      void spellDetectLanguage(text).then((lang) => {
        if (lang && path === project.view.main) project.detectedLanguage = lang
      })
    }
  }, 400)
}

function onUpdate(path: string, state: EditorState, docChanged: boolean) {
  buffers.setState(path, state)
  const head = state.selection.main.head
  const line = state.doc.lineAt(head)
  editor.cursor.line = line.number
  editor.cursor.col = head - line.from + 1
  if (docChanged) {
    editor.onDocChanged(path)
    scheduleIndex(path)
  }
  clearTimeout(selTimer)
  const sel = state.selection.main
  if (sel.empty) {
    editor.cursor.selectionWords = null
  } else {
    const text = state.sliceDoc(sel.from, sel.to)
    selTimer = setTimeout(() => {
      void wordCountText(text).then((c) => (editor.cursor.selectionWords = c.words + c.headers))
    }, 250)
  }
}

function stateFor(path: string): EditorState {
  const existing = buffers.getState(path)
  if (existing) return existing
  const doc = buffers.takePending(path) ?? ''
  const state = EditorState.create({
    doc,
    extensions: editorExtensions({
      path,
      prefs: { wrap: editor.prefs.wrap, locale: locale.value },
      completionSources: latexCompletionSources(completionData),
      extra: [
        EditorView.updateListener.of((u) => onUpdate(path, u.state, u.docChanged)),
        EditorView.domEventHandlers({ blur: () => editor.onBlur() }),
        refsExtension({
          view: () => project.view,
          file: () => path,
          t: (key, params) => t(key, params),
          onGoto: (name) => void gotoLabel(name),
          onUsages: (name) => showUsages(name),
          onRename: (name) => (refs.renaming = name),
        }),
        compileLintExtension({
          diagnostics: () =>
            project.root
              ? compile.diagnostics.filter(
                  (d) => d.file !== null && sameFile(project.root as string, d.file, path),
                )
              : [],
          showBoxes: () => compile.showBoxes,
          t: (key) => t(key),
          has: (key) => te(key),
        }),
        spellExtension({
          lang: spellLang,
          check: spellCheck,
          onMenu: (info) => void openSpellMenu(info),
        }),
      ],
    }),
  })
  buffers.setState(path, state)
  return state
}

async function openSpellMenu(info: Omit<SpellMenu, 'suggestions'>) {
  const lang = spellLang()
  let suggestions: string[] = []
  if (lang) {
    try {
      suggestions = await spellSuggest(info.word, lang)
    } catch {
      suggestions = []
    }
  }
  spellMenu.value = { ...info, suggestions }
}

function applySuggestion(word: string) {
  const m = spellMenu.value
  if (!m || !view) return
  view.dispatch({ changes: { from: m.from, to: m.to, insert: word }, userEvent: 'input.spell' })
  spellMenu.value = null
  view.focus()
}

async function addWord(scope: 'global' | 'project') {
  const m = spellMenu.value
  if (!m) return
  spellMenu.value = null
  try {
    await spellAddWord(m.word, scope, project.root)
    if (view) recheck(view)
  } catch {
    // sem projeto aberto para o escopo "project"
  }
}

function show() {
  if (!view) return
  const path = editor.active
  if (!path || editor.activeTab?.media) {
    registerActiveView(null, null)
    return
  }
  view.setState(stateFor(path))
  view.dispatch({ effects: wrapEffect(editor.prefs.wrap) })
  registerActiveView(view, flushIndex)
  applyReveal()
  view.focus()
}

function applyReveal() {
  const r = editor.reveal
  if (
    !view ||
    !r ||
    !editor.active ||
    r.path.replace(/\\/g, '/') !== editor.active.replace(/\\/g, '/')
  ) {
    return
  }
  const doc = view.state.doc
  const line = doc.line(Math.min(Math.max(r.line + 1, 1), doc.lines))
  const from = line.from + Math.min(r.col, line.length)
  const to = r.endCol !== undefined ? line.from + Math.min(r.endCol, line.length) : from
  view.dispatch({
    selection: EditorSelection.range(from, to),
    effects: EditorView.scrollIntoView(from, { y: 'center' }),
  })
  view.focus()
}

onMounted(() => {
  view = new EditorView({
    parent: host.value as HTMLElement,
    state: EditorState.create({ doc: '' }),
  })
  window.addEventListener('blur', onWindowBlur)
  show()
})

function onWindowBlur() {
  editor.onBlur()
}

onBeforeUnmount(() => {
  window.removeEventListener('blur', onWindowBlur)
  clearTimeout(indexTimer)
  clearTimeout(selTimer)
  registerActiveView(null, null)
  view?.destroy()
})

watch(() => [editor.active, editor.reloadTick], show)
watch(() => editor.reveal, applyReveal)
watch(
  () => editor.prefs.wrap,
  (wrap) => view?.dispatch({ effects: wrapEffect(wrap) }),
)
watch(
  () => [compile.outcome, compile.showBoxes],
  () => view && forceLinting(view),
)
watch(
  () => project.spellLanguage,
  () => view && recheck(view),
)
</script>

<template>
  <div class="wrap">
    <div ref="host" class="host" :class="{ hidden: !editor.active }" />
    <div
      v-if="spellMenu"
      class="backdrop"
      @mousedown="spellMenu = null"
      @contextmenu.prevent="spellMenu = null"
    />
    <ul
      v-if="spellMenu"
      class="menu"
      role="menu"
      :style="{ left: spellMenu.x + 'px', top: spellMenu.y + 'px' }"
    >
      <li v-for="s in spellMenu.suggestions" :key="s" role="menuitem" @click="applySuggestion(s)">
        {{ s }}
      </li>
      <li v-if="!spellMenu.suggestions.length" class="muted">
        {{ t('editor.spell.noSuggestions') }}
      </li>
      <li class="sep" role="separator" />
      <li v-if="project.root" role="menuitem" @click="addWord('project')">
        {{ t('editor.spell.addProject') }}
      </li>
      <li role="menuitem" @click="addWord('global')">{{ t('editor.spell.addGlobal') }}</li>
    </ul>
  </div>
</template>

<style scoped>
.wrap {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
}
.host {
  flex: 1;
  min-width: 0;
  min-height: 0;
}
.host.hidden {
  display: none;
}
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 40;
}
.menu {
  position: fixed;
  z-index: 41;
  margin: 0;
  padding: var(--space-1);
  min-width: 200px;
  list-style: none;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
}
.menu li {
  padding: 5px var(--space-3);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.menu li:hover:not(.sep):not(.muted) {
  background: var(--bg-hover);
}
.menu .muted {
  color: var(--text-muted);
  cursor: default;
}
.menu .sep {
  height: 1px;
  margin: var(--space-1) 0;
  padding: 0;
  background: var(--border);
  cursor: default;
}
.host :deep(.cm-editor) {
  height: 100%;
}
</style>
