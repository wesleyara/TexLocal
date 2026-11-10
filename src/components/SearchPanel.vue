<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  projectReplace,
  projectSearch,
  relativePath,
  type FileMatches,
  type SearchQuery,
} from '../project/api'
import { samePath } from '../project/model'
import { useEditorStore } from '../stores/editor'
import { useNotificationsStore } from '../stores/notifications'
import { useProjectStore } from '../stores/project'
import { useUiStore } from '../stores/ui'
import { isTauri } from '../tauri'

const { t } = useI18n()
const project = useProjectStore()
const editor = useEditorStore()
const notify = useNotificationsStore()
const ui = useUiStore()

const pattern = ref('')
const replacement = ref('')
const useRegex = ref(false)
const caseSensitive = ref(false)
const wholeWord = ref(false)
const glob = ref('')
const results = ref<FileMatches[]>([])
const busy = ref(false)
const error = ref('')
const searched = ref(false)
const input = ref<HTMLInputElement | null>(null)

const total = computed(() => results.value.reduce((n, f) => n + f.matches.length, 0))

function query(): SearchQuery {
  return {
    pattern: pattern.value,
    regex: useRegex.value,
    caseSensitive: caseSensitive.value,
    wholeWord: wholeWord.value,
    glob: glob.value,
  }
}

/** Fora do modo regex o `$` da substituição é literal. */
const effectiveReplacement = () =>
  useRegex.value ? replacement.value : replacement.value.replace(/\$/g, '$$$$')

async function run() {
  if (!project.root || !pattern.value) return
  busy.value = true
  error.value = ''
  try {
    results.value = await projectSearch(project.root, query(), effectiveReplacement())
    searched.value = true
  } catch (e) {
    error.value = String(e)
    results.value = []
  } finally {
    busy.value = false
  }
}

/**
 * Confirmação nativa. No app desktop o `window.confirm` é substituído por uma versão assíncrona
 * (devolve uma Promise, sempre "verdadeira"), então a substituição rodava sem esperar o "OK".
 */
async function confirmReplace(message: string): Promise<boolean> {
  if (isTauri()) {
    const { ask } = await import('@tauri-apps/plugin-dialog')
    return ask(message, { title: t('search.title'), kind: 'warning' })
  }
  return window.confirm(message)
}

// A prévia da substituição vem do backend: refaz a busca quando o texto novo muda.
let previewTimer: ReturnType<typeof setTimeout> | undefined
watch(replacement, () => {
  clearTimeout(previewTimer)
  if (!searched.value || !pattern.value) return
  previewTimer = setTimeout(() => void run(), 250)
})

async function replaceIn(files: string[]) {
  const dirty = files.filter((f) => editor.tabs.some((tab) => tab.dirty && samePath(tab.path, f)))
  const safe = files.filter((f) => !dirty.includes(f))
  if (dirty.length) notify.notify('info', t('search.skippedDirty', { count: dirty.length }), 8000)
  if (!safe.length) return
  if (!(await confirmReplace(t('search.confirm', { count: safe.length })))) return
  try {
    const changed = await projectReplace(query(), effectiveReplacement(), safe)
    for (const f of changed) await editor.reloadFromDisk(f)
    if (project.root) await project.open(project.root)
    notify.notify('success', t('search.replaced', { count: changed.length }))
    await run()
  } catch (e) {
    notify.notify('error', String(e))
  }
}

const showPreview = (m: { replaced: string | null }) => !!replacement.value && m.replaced !== null

/** Arquivos recolhidos na lista de resultados. */
const collapsed = ref(new Set<string>())
function toggleFile(path: string) {
  const next = new Set(collapsed.value)
  if (!next.delete(path)) next.add(path)
  collapsed.value = next
}

function splitPath(path: string): { dir: string; name: string } {
  const rel = project.root ? relativePath(project.root, path) : path
  const i = rel.lastIndexOf('/')
  return i < 0 ? { dir: '', name: rel } : { dir: rel.slice(0, i), name: rel.slice(i + 1) }
}

/** Trecho da linha em torno do achado, para caber numa linha da barra lateral. */
function snippet(m: { col: number; endCol: number; text: string }) {
  const text = m.text
  const lead = text.length - text.trimStart().length
  const start = Math.min(Math.max(m.col, lead), text.length)
  const end = Math.min(Math.max(m.endCol, start), text.length)
  const from = Math.max(lead, start - 28)
  return {
    cut: from > lead,
    before: text.slice(from, start),
    hit: text.slice(start, end),
    after: text.slice(end).trimEnd(),
  }
}

function openMatch(file: string, m: { line: number; col: number; endCol: number }) {
  void editor.openFile(file, m)
}

watch(
  () => ui.searchFocusTick,
  async () => {
    await nextTick()
    input.value?.focus()
    input.value?.select()
  },
)
</script>

<template>
  <section class="search" :aria-label="t('search.title')">
    <h2>{{ t('search.title') }}</h2>
    <form @submit.prevent="run">
      <input
        ref="input"
        v-model="pattern"
        :placeholder="t('search.find')"
        :aria-label="t('search.find')"
      />
      <input
        v-model="replacement"
        :placeholder="t('search.replace')"
        :aria-label="t('search.replace')"
      />
      <input v-model="glob" :placeholder="t('search.glob')" :aria-label="t('search.glob')" />
      <div class="toggles">
        <label><input v-model="useRegex" type="checkbox" /> {{ t('search.regex') }}</label>
        <label><input v-model="caseSensitive" type="checkbox" /> {{ t('search.case') }}</label>
        <label><input v-model="wholeWord" type="checkbox" /> {{ t('search.word') }}</label>
      </div>
      <div class="actions">
        <button type="submit" :disabled="busy || !pattern || !project.root">
          {{ t('search.run') }}
        </button>
        <button type="button" :disabled="!total" @click="replaceIn(results.map((r) => r.path))">
          {{ t('search.replaceAll') }}
        </button>
      </div>
    </form>
    <p v-if="error" class="error" role="alert">{{ error }}</p>
    <p v-else-if="searched" class="muted" role="status">
      {{ t('search.summary', { matches: total, files: results.length }) }}
    </p>
    <ul class="results">
      <li v-for="f in results" :key="f.path" class="group">
        <div class="file">
          <button
            class="head"
            :title="f.path"
            :aria-expanded="!collapsed.has(f.path)"
            @click="toggleFile(f.path)"
          >
            <span class="chev" aria-hidden="true">{{ collapsed.has(f.path) ? '▸' : '▾' }}</span>
            <span class="fname">{{ splitPath(f.path).name }}</span>
            <span v-if="splitPath(f.path).dir" class="fdir">{{ splitPath(f.path).dir }}</span>
            <span class="count">{{ f.matches.length }}</span>
          </button>
          <button class="small" :title="t('search.replaceFile')" @click="replaceIn([f.path])">
            ⇄
          </button>
        </div>
        <ul v-show="!collapsed.has(f.path)">
          <li
            v-for="(m, i) in f.matches"
            :key="i"
            class="match"
            tabindex="0"
            @click="openMatch(f.path, m)"
            @keydown.enter="openMatch(f.path, m)"
          >
            <span class="ln">{{ m.line + 1 }}</span>
            <span class="txt" :title="m.text.trim()"
              ><template v-if="snippet(m).cut">…</template>{{ snippet(m).before
              }}<mark :class="{ old: showPreview(m) }">{{ snippet(m).hit }}</mark
              ><ins v-if="showPreview(m) && !useRegex">{{ replacement }}</ins
              >{{ snippet(m).after }}</span
            >
            <span v-if="showPreview(m) && useRegex" class="new" :title="m.replaced?.trim()"
              >→ {{ m.replaced?.trim() }}</span
            >
          </li>
        </ul>
      </li>
    </ul>
  </section>
</template>

<style scoped>
h2 {
  margin: 0 0 var(--space-2);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
}
form {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
input[type='text'],
form > input {
  width: 100%;
  padding: 5px var(--space-2);
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}
.toggles {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  color: var(--text-muted);
  font-size: 12px;
}
.actions {
  display: flex;
  gap: var(--space-2);
}
button {
  padding: 4px var(--space-3);
  background: var(--bg-hover);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
button:disabled {
  opacity: 0.5;
  cursor: default;
}
.small {
  padding: 0 6px;
}
.muted {
  color: var(--text-muted);
  margin: var(--space-2) 0;
}
.error {
  color: var(--danger);
}
ul {
  margin: 0;
  padding: 0;
  list-style: none;
}
.group {
  margin-top: var(--space-2);
}
.file {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-1);
  padding: 2px 0;
  position: sticky;
  top: 0;
  background: var(--bg-panel);
}
.head {
  display: flex;
  align-items: baseline;
  gap: var(--space-1);
  min-width: 0;
  flex: 1;
  padding: 2px 0;
  background: none;
  border: 0;
  text-align: left;
}
.chev {
  width: 12px;
  color: var(--text-muted);
  flex: none;
}
.fname {
  font-weight: 600;
  white-space: nowrap;
  flex: none;
}
.fdir {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-muted);
  font-size: 11px;
}
.count {
  margin-left: auto;
  flex: none;
  padding: 0 6px;
  border-radius: 8px;
  background: var(--bg-hover);
  color: var(--text-muted);
  font-size: 11px;
}
.match {
  display: grid;
  grid-template-columns: 34px minmax(0, 1fr);
  column-gap: var(--space-2);
  padding: 2px 4px 2px 16px;
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.match:hover,
.match:focus-visible {
  background: var(--bg-hover);
}
.ln {
  color: var(--text-muted);
  text-align: right;
  font-size: 11px;
  line-height: 18px;
}
.txt,
.new {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: pre;
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 18px;
}
mark {
  padding: 0 1px;
  border-radius: 2px;
  background: rgb(255 200 0 / 35%);
  color: inherit;
}
ins {
  padding: 0 1px;
  border-radius: 2px;
  background: rgb(80 200 120 / 30%);
  color: inherit;
  text-decoration: none;
}
mark.old {
  background: rgb(255 90 90 / 30%);
  text-decoration: line-through;
}
.new {
  grid-column: 2;
  color: var(--success);
}
</style>
