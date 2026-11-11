<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { snippetPreview } from '../editor/completions'
import { searchSnippets, type Snippet, type SnippetScope } from '../snippets/model'
import { useNotificationsStore } from '../stores/notifications'
import { useSnippetsStore } from '../stores/snippets'
import { useUiStore } from '../stores/ui'

const { t } = useI18n()
const ui = useUiStore()
const store = useSnippetsStore()
const notifications = useNotificationsStore()

type Filter = 'all' | SnippetScope

const query = ref('')
const filter = ref<Filter>('all')
const selected = ref<Snippet | null>(null)

interface Form {
  name: string
  prefixes: string
  body: string
  description: string
  languages: string
  keybinding: string
  scope: 'global' | 'project'
}
const emptyForm = (): Form => ({
  name: '',
  prefixes: '',
  body: '',
  description: '',
  languages: 'latex',
  keybinding: '',
  scope: store.dirFor('project') ? 'project' : 'global',
})
const form = ref<Form>(emptyForm())

const list = computed(() => {
  const base = store.everything.filter((s) => filter.value === 'all' || s.scope === filter.value)
  return searchSnippets(base, query.value)
})

const preview = computed(() => snippetPreview(form.value.body))
const readOnly = computed(() => selected.value?.scope === 'builtin')
const canSave = computed(
  () => !!form.value.name.trim() && !!form.value.prefixes.trim() && !!form.value.body,
)

watch(
  () => ui.snippetsOpen,
  (open) => {
    if (!open) return
    query.value = ''
    filter.value = 'all'
    selected.value = null
    form.value = emptyForm()
    if (ui.snippetDraft !== null) form.value.body = ui.snippetDraft
  },
)

function pick(s: Snippet) {
  selected.value = s
  form.value = {
    name: s.name,
    prefixes: s.prefixes.join(', '),
    body: s.body,
    description: s.description,
    languages: s.languages.join(', '),
    keybinding: s.keybinding ?? '',
    scope: s.scope === 'project' ? 'project' : 'global',
  }
}

function newSnippet() {
  selected.value = null
  form.value = emptyForm()
}

function list2(text: string): string[] {
  return text
    .split(',')
    .map((x) => x.trim())
    .filter(Boolean)
}

async function save() {
  const f = form.value
  // Edição de um snippet do padrão embutido gera uma cópia (que o sobrescreve pelo prefixo).
  const previous = selected.value && selected.value.scope !== 'builtin' ? selected.value : undefined
  const snippet: Snippet = {
    id: '',
    name: f.name.trim(),
    prefixes: list2(f.prefixes),
    body: f.body,
    description: f.description.trim(),
    languages: list2(f.languages),
    scope: f.scope,
    category: '',
    file: '',
    keybinding: f.keybinding.trim() || undefined,
  }
  try {
    await store.save(f.scope, snippet, previous)
    notifications.notify('success', t('snippets.saved'))
    selected.value =
      store.everything.find((s) => s.scope === f.scope && s.name === snippet.name) ?? null
  } catch (e) {
    notifications.notify('error', String((e as Error).message ?? e))
  }
}

async function remove() {
  const s = selected.value
  if (!s || s.scope === 'builtin') return
  try {
    await store.remove(s)
    newSnippet()
  } catch (e) {
    notifications.notify('error', String((e as Error).message ?? e))
  }
}

async function doImport() {
  try {
    const n = await store.importFiles(form.value.scope)
    if (n) notifications.notify('success', t('snippets.imported', { n }))
  } catch (e) {
    notifications.notify('error', String((e as Error).message ?? e))
  }
}

async function doExport() {
  try {
    const items = list.value.filter((s) => s.scope !== 'builtin')
    const target = await store.exportSnippets(items.length ? items : list.value)
    if (target) notifications.notify('success', t('snippets.exported', { path: target }))
  } catch (e) {
    notifications.notify('error', String((e as Error).message ?? e))
  }
}

function openJson() {
  const scope = selected.value?.scope === 'project' ? 'project' : form.value.scope
  close()
  void store.openJson(scope, selected.value ?? undefined)
}

function close() {
  ui.snippetsOpen = false
  ui.snippetDraft = null
}
</script>

<template>
  <div v-if="ui.snippetsOpen" class="overlay" @mousedown.self="close">
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      :aria-label="t('snippets.title')"
      @keydown.esc="close"
    >
      <header>
        <h1>{{ t('snippets.title') }}</h1>
        <button :aria-label="t('settings.close')" @click="close">×</button>
      </header>
      <div class="cols">
        <section class="left">
          <input v-model="query" type="search" :placeholder="t('snippets.search')" />
          <select v-model="filter" :aria-label="t('snippets.filter')">
            <option value="all">{{ t('snippets.scope.all') }}</option>
            <option value="builtin">{{ t('snippets.scope.builtin') }}</option>
            <option value="global">{{ t('snippets.scope.global') }}</option>
            <option value="project">{{ t('snippets.scope.project') }}</option>
          </select>
          <ul>
            <li
              v-for="s in list"
              :key="s.id"
              :class="{ on: selected?.id === s.id }"
              tabindex="0"
              @click="pick(s)"
              @keydown.enter="pick(s)"
            >
              <strong>{{ s.name }}</strong>
              <kbd>{{ s.prefixes.join(' · ') }}</kbd>
              <small>{{ t(`snippets.scope.${s.scope}`) }}</small>
            </li>
            <li v-if="!list.length" class="muted">{{ t('palette.noResults') }}</li>
          </ul>
          <div class="actions">
            <button @click="newSnippet">{{ t('snippets.new') }}</button>
            <button @click="doImport">{{ t('snippets.import') }}</button>
            <button @click="doExport">{{ t('snippets.export') }}</button>
          </div>
        </section>

        <section class="right">
          <p v-if="readOnly" class="note">{{ t('snippets.builtinNote') }}</p>
          <label>
            {{ t('snippets.name') }}
            <input v-model="form.name" type="text" />
          </label>
          <label>
            {{ t('snippets.prefix') }}
            <input v-model="form.prefixes" type="text" placeholder="fig, figure" />
          </label>
          <label>
            {{ t('snippets.description') }}
            <input v-model="form.description" type="text" />
          </label>
          <label>
            {{ t('snippets.body') }}
            <textarea v-model="form.body" rows="9" spellcheck="false" />
          </label>
          <div class="pair">
            <label>
              {{ t('snippets.languages') }}
              <input v-model="form.languages" type="text" placeholder="latex, bibtex" />
            </label>
            <label>
              {{ t('snippets.keybinding') }}
              <input v-model="form.keybinding" type="text" placeholder="Mod+Alt+F" />
            </label>
            <label>
              {{ t('snippets.saveIn') }}
              <select v-model="form.scope">
                <option value="global">{{ t('snippets.scope.global') }}</option>
                <option value="project" :disabled="!store.dirFor('project')">
                  {{ t('snippets.scope.project') }}
                </option>
              </select>
            </label>
          </div>
          <h2>{{ t('snippets.preview') }}</h2>
          <pre class="preview">{{ preview }}</pre>
          <footer>
            <button class="primary" :disabled="!canSave" @click="save">
              {{ readOnly ? t('snippets.saveCopy') : t('snippets.save') }}
            </button>
            <button v-if="selected && !readOnly" @click="remove">{{ t('snippets.delete') }}</button>
            <button @click="openJson">{{ t('snippets.openJson') }}</button>
          </footer>
        </section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  justify-content: center;
  align-items: center;
  background: rgb(0 0 0 / 35%);
}
.dialog {
  display: flex;
  flex-direction: column;
  width: min(900px, 94vw);
  height: min(640px, 88vh);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
}
header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
}
h1 {
  margin: 0;
  font-size: 15px;
}
h2 {
  margin: var(--space-3) 0 var(--space-1);
  font-size: 12px;
  text-transform: uppercase;
  color: var(--text-muted);
}
header button {
  background: none;
  border: 0;
  font-size: 20px;
  cursor: pointer;
  color: var(--text);
}
.cols {
  display: flex;
  flex: 1;
  min-height: 0;
}
.left {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  width: 300px;
  padding: var(--space-3);
  border-right: 1px solid var(--border);
}
.right {
  flex: 1;
  min-width: 0;
  padding: var(--space-3) var(--space-4);
  overflow: auto;
}
ul {
  flex: 1;
  margin: 0;
  padding: 0;
  overflow: auto;
  list-style: none;
}
li {
  display: flex;
  flex-wrap: wrap;
  gap: 0 var(--space-2);
  align-items: baseline;
  padding: 4px var(--space-2);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
li:hover {
  background: var(--bg-hover);
}
li.on {
  background: var(--accent);
  color: var(--accent-contrast);
}
li small,
li kbd {
  color: inherit;
  opacity: 0.7;
  font-family: var(--font-ui);
}
li small {
  margin-left: auto;
}
.muted {
  color: var(--text-muted);
  cursor: default;
}
.actions,
footer {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
footer {
  margin-top: var(--space-3);
}
label {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-bottom: var(--space-2);
  color: var(--text-muted);
  font-size: 12px;
}
.pair {
  display: flex;
  gap: var(--space-3);
}
.pair label {
  flex: 1;
}
input,
select,
textarea,
button {
  padding: 4px var(--space-2);
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: 13px;
}
button {
  cursor: pointer;
}
button.primary {
  background: var(--accent);
  color: var(--accent-contrast);
  border-color: var(--accent);
}
button:disabled {
  opacity: 0.5;
  cursor: default;
}
textarea,
.preview {
  font-family: var(--font-mono);
}
.preview {
  margin: 0;
  padding: var(--space-2);
  max-height: 140px;
  overflow: auto;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}
.note {
  margin: 0 0 var(--space-2);
  color: var(--text-muted);
}
</style>
