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

async function replaceIn(files: string[]) {
  const dirty = files.filter((f) => editor.tabs.some((tab) => tab.dirty && samePath(tab.path, f)))
  const safe = files.filter((f) => !dirty.includes(f))
  if (dirty.length) notify.notify('info', t('search.skippedDirty', { count: dirty.length }), 8000)
  if (!safe.length) return
  if (!window.confirm(t('search.confirm', { count: safe.length }))) return
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
      <li v-for="f in results" :key="f.path">
        <div class="file">
          <span class="fname" :title="f.path">{{
            project.root ? relativePath(project.root, f.path) : f.path
          }}</span>
          <button class="small" :title="t('search.replaceFile')" @click="replaceIn([f.path])">
            ⇄
          </button>
        </div>
        <ul>
          <li
            v-for="(m, i) in f.matches"
            :key="i"
            class="match"
            tabindex="0"
            @click="openMatch(f.path, m)"
            @keydown.enter="openMatch(f.path, m)"
          >
            <span class="ln">{{ m.line + 1 }}</span>
            <span class="txt">{{ m.text.trim() }}</span>
            <span v-if="replacement && m.replaced !== null" class="new"
              >→ {{ m.replaced.trim() }}</span
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
.results > li {
  margin-top: var(--space-2);
}
.file {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
}
.fname {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.match {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  padding: 2px 4px;
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.match:hover {
  background: var(--bg-hover);
}
.ln {
  color: var(--text-muted);
  min-width: 24px;
  text-align: right;
}
.txt {
  font-family: var(--font-mono);
  font-size: 12px;
}
.new {
  width: 100%;
  padding-left: 32px;
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--success);
}
</style>
