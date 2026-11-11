<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  confirmDialog,
  historyChanged,
  historySides,
  type CommitInfo,
  type FileDiff,
  type FileSides,
} from '../history/api'
import { buildRows, hunksOf, revertHunk, withContext, type Row, type Seg } from '../history/diff'
import { commitTitle } from '../history/model'
import { useHistoryStore, type CompareEnd } from '../stores/history'
import { useProjectStore } from '../stores/project'
import AppIcon from './AppIcon.vue'

const { t, locale } = useI18n()
const history = useHistoryStore()
const project = useProjectStore()

const files = ref<FileDiff[]>([])
const selected = ref<string | null>(null)
const sides = ref<FileSides | null>(null)
const error = ref('')
const loading = ref(false)
const mode = ref<'inline' | 'side'>('inline')
const showAll = ref(false)

const cmp = computed(() => history.comparison)
const fromRev = computed(() => cmp.value?.from?.id ?? null)
const toRev = computed(() => (cmp.value && cmp.value.to !== 'workdir' ? cmp.value.to.id : null))
const toWorkdir = computed(() => cmp.value?.to === 'workdir')

/** Opções dos seletores: commits carregados + as pontas atuais (podem estar fora da página). */
const options = computed(() => {
  const list: CommitInfo[] = [...history.commits]
  const c = cmp.value
  for (const e of [c?.from, c?.to]) {
    if (e && e !== 'workdir' && !list.some((x) => x.id === e.id)) list.push(e)
  }
  return list
})

function label(c: CommitInfo): string {
  const when = c.time
    ? new Date(c.time * 1000).toLocaleString(locale.value, {
        dateStyle: 'short',
        timeStyle: 'short',
      })
    : ''
  return `${commitTitle(c)}${when ? ` · ${when}` : ''}`
}

function setFrom(id: string) {
  const c = cmp.value
  if (!c) return
  history.compare(id ? (options.value.find((x) => x.id === id) ?? null) : null, c.to)
}
function setTo(id: string) {
  const c = cmp.value
  if (!c) return
  const to: CompareEnd =
    id === 'workdir' ? 'workdir' : (options.value.find((x) => x.id === id) ?? 'workdir')
  history.compare(c.from, to)
}

async function loadFiles() {
  files.value = []
  selected.value = null
  sides.value = null
  error.value = ''
  if (!cmp.value || !project.root) return
  loading.value = true
  try {
    files.value = await historyChanged(project.root, fromRev.value, toRev.value)
    if (files.value.length) selected.value = files.value[0].path
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

async function loadSides() {
  sides.value = null
  if (!selected.value || !project.root) return
  try {
    sides.value = await historySides(project.root, fromRev.value, toRev.value, selected.value)
  } catch (e) {
    error.value = String(e)
  }
}

watch(
  () => [cmp.value?.from?.id, cmp.value && (cmp.value.to === 'workdir' ? 'w' : cmp.value.to.id)],
  loadFiles,
  {
    immediate: true,
  },
)
watch(selected, loadSides)

const rows = computed<Row[]>(() => {
  const s = sides.value
  if (!s || s.binary) return []
  return buildRows(s.oldText ?? '', s.newText ?? '')
})
const hunks = computed(() => hunksOf(rows.value))
const hunkAt = computed(() => new Map(hunks.value.map((h, i) => [h.start, i])))
const visible = computed(() =>
  showAll.value ? rows.value.map((row, index) => ({ row, index })) : withContext(rows.value, 3),
)

const image = (data: string | null, mime: string | null) =>
  data && mime ? `data:${mime};base64,${data}` : null

const totals = computed(() => ({
  add: files.value.reduce((n, f) => n + f.additions, 0),
  del: files.value.reduce((n, f) => n + f.deletions, 0),
}))

const canRestore = computed(() => !!cmp.value?.from)
const fromName = computed(() => (cmp.value?.from ? commitTitle(cmp.value.from) : ''))

async function restoreFile() {
  const from = cmp.value?.from
  if (!from || !selected.value) return
  const ok = await confirmDialog(
    t('history.confirmRestoreFile', { file: selected.value, version: commitTitle(from) }),
    t('history.title'),
  )
  if (ok) {
    await history.restore(from.id, [selected.value])
    await loadFiles()
  }
}

async function restoreProject() {
  const from = cmp.value?.from
  if (!from || !project.root) return
  const affected = await historyChanged(project.root, from.id, null).catch(() => [])
  const list = affected
    .slice(0, 12)
    .map((f) => `• ${f.path} (${t(`history.status.${f.status}`)})`)
    .join('\n')
  const more = affected.length > 12 ? `\n… +${affected.length - 12}` : ''
  const ok = await confirmDialog(
    t('history.confirmRestoreProject', {
      version: commitTitle(from),
      n: affected.length,
      list: list + more,
    }),
    t('history.title'),
  )
  if (ok) {
    await history.restore(from.id, null)
    await loadFiles()
  }
}

async function restoreHunk(index: number) {
  const from = cmp.value?.from
  if (!from || !selected.value) return
  const ok = await confirmDialog(
    t('history.confirmRestoreHunk', { file: selected.value, version: commitTitle(from) }),
    t('history.title'),
  )
  if (!ok) return
  await history.restoreText(from.id, selected.value, revertHunk(rows.value, hunks.value, index))
  await loadFiles()
}

const parts = (segs: Seg[] | undefined, fallback: string | null) =>
  segs ?? [{ text: fallback ?? '', changed: false }]
</script>

<template>
  <div v-if="cmp" class="overlay" @mousedown.self="history.closeComparison()">
    <section
      class="dialog"
      role="dialog"
      aria-modal="true"
      :aria-label="t('history.diffTitle')"
      tabindex="-1"
      @keydown.esc.stop="history.closeComparison()"
    >
      <header>
        <h3>{{ t('history.diffTitle') }}</h3>
        <label>
          {{ t('history.from') }}
          <select
            :value="fromRev ?? ''"
            @change="setFrom(($event.target as HTMLSelectElement).value)"
          >
            <option value="">{{ t('history.emptyProject') }}</option>
            <option v-for="c in options" :key="c.id" :value="c.id">{{ label(c) }}</option>
          </select>
        </label>
        <label>
          {{ t('history.to') }}
          <select
            :value="toWorkdir ? 'workdir' : toRev"
            @change="setTo(($event.target as HTMLSelectElement).value)"
          >
            <option value="workdir">{{ t('history.current') }}</option>
            <option v-for="c in options" :key="c.id" :value="c.id">{{ label(c) }}</option>
          </select>
        </label>
        <span class="spacer" />
        <button
          class="icon-btn"
          :aria-label="t('history.close')"
          :title="t('history.close')"
          @click="history.closeComparison()"
        >
          <AppIcon name="close" :size="16" />
        </button>
      </header>

      <div class="body">
        <aside class="files">
          <div class="summary">
            {{ t('history.files', { n: files.length }) }}
            <span class="add">+{{ totals.add }}</span> <span class="del">−{{ totals.del }}</span>
          </div>
          <p v-if="loading" class="muted">…</p>
          <p v-else-if="!files.length && !error" class="muted">{{ t('history.noDifferences') }}</p>
          <ul>
            <li v-for="f in files" :key="f.path">
              <button
                :class="{ active: f.path === selected }"
                :title="f.path"
                @click="selected = f.path"
              >
                <span :class="['st', f.status]">{{ f.status[0].toUpperCase() }}</span>
                <span class="path">{{ f.path }}</span>
                <span v-if="f.binary" class="muted">bin</span>
                <template v-else>
                  <span class="add">+{{ f.additions }}</span>
                  <span class="del">−{{ f.deletions }}</span>
                </template>
              </button>
            </li>
          </ul>
          <div class="restore">
            <button :disabled="!canRestore || history.busy" @click="restoreProject">
              <AppIcon name="restore" :size="14" /> {{ t('history.restoreProject') }}
            </button>
          </div>
        </aside>

        <div class="view">
          <p v-if="error" class="error">{{ error }}</p>
          <div v-if="selected" class="toolbar">
            <strong class="path">{{ selected }}</strong>
            <span class="spacer" />
            <label v-if="sides && !sides.binary" class="row">
              <input v-model="showAll" type="checkbox" /> {{ t('history.showAll') }}
            </label>
            <div v-if="sides && !sides.binary" class="seg" role="group">
              <button :class="{ on: mode === 'inline' }" @click="mode = 'inline'">
                {{ t('history.inline') }}
              </button>
              <button :class="{ on: mode === 'side' }" @click="mode = 'side'">
                {{ t('history.sideBySide') }}
              </button>
            </div>
            <button :disabled="!canRestore || history.busy" :title="fromName" @click="restoreFile">
              <AppIcon name="restore" :size="14" /> {{ t('history.restoreFile') }}
            </button>
          </div>

          <div v-if="sides?.binary" class="binary">
            <template v-if="image(sides.oldData, sides.mime) || image(sides.newData, sides.mime)">
              <figure>
                <figcaption>{{ t('history.before') }}</figcaption>
                <img
                  v-if="image(sides.oldData, sides.mime)"
                  :src="image(sides.oldData, sides.mime)!"
                  alt=""
                />
                <span v-else class="muted">—</span>
              </figure>
              <figure>
                <figcaption>{{ t('history.after') }}</figcaption>
                <img
                  v-if="image(sides.newData, sides.mime)"
                  :src="image(sides.newData, sides.mime)!"
                  alt=""
                />
                <span v-else class="muted">—</span>
              </figure>
            </template>
            <p v-else class="muted">{{ t('history.binary') }}</p>
          </div>

          <div v-else-if="sides" class="code" :class="mode">
            <template v-for="(v, i) in visible" :key="i">
              <div v-if="'gap' in v" class="gap">⋯ {{ t('history.unchanged', { n: v.gap }) }}</div>
              <template v-else>
                <div v-if="hunkAt.has(v.index) && toWorkdir && canRestore" class="hunk-bar">
                  <button
                    class="link"
                    :disabled="history.busy"
                    @click="restoreHunk(hunkAt.get(v.index)!)"
                  >
                    <AppIcon name="restore" :size="12" /> {{ t('history.restoreHunk') }}
                  </button>
                </div>
                <div v-if="mode === 'inline'" class="inline">
                  <template v-if="v.row.kind === 'equal'">
                    <div class="ln eq">
                      <span class="no">{{ v.row.oldNo }}</span
                      ><span class="no">{{ v.row.newNo }}</span>
                      <span class="tx">{{ v.row.new }}</span>
                    </div>
                  </template>
                  <template v-else>
                    <div v-if="v.row.old !== null" class="ln rem">
                      <span class="no">{{ v.row.oldNo }}</span
                      ><span class="no" />
                      <span class="tx">
                        <span
                          v-for="(s, k) in parts(v.row.oldParts, v.row.old)"
                          :key="k"
                          :class="{ w: s.changed && v.row.kind === 'change' }"
                          >{{ s.text }}</span
                        >
                      </span>
                    </div>
                    <div v-if="v.row.new !== null" class="ln add">
                      <span class="no" /><span class="no">{{ v.row.newNo }}</span>
                      <span class="tx">
                        <span
                          v-for="(s, k) in parts(v.row.newParts, v.row.new)"
                          :key="k"
                          :class="{ w: s.changed && v.row.kind === 'change' }"
                          >{{ s.text }}</span
                        >
                      </span>
                    </div>
                  </template>
                </div>
                <div v-else class="split">
                  <div
                    class="ln half"
                    :class="v.row.old === null ? 'empty' : v.row.kind === 'equal' ? 'eq' : 'rem'"
                  >
                    <span class="no">{{ v.row.oldNo }}</span>
                    <span class="tx">
                      <span
                        v-for="(s, k) in parts(v.row.oldParts, v.row.old)"
                        :key="k"
                        :class="{ w: s.changed && v.row.kind === 'change' }"
                        >{{ s.text }}</span
                      >
                    </span>
                  </div>
                  <div
                    class="ln half"
                    :class="v.row.new === null ? 'empty' : v.row.kind === 'equal' ? 'eq' : 'add'"
                  >
                    <span class="no">{{ v.row.newNo }}</span>
                    <span class="tx">
                      <span
                        v-for="(s, k) in parts(v.row.newParts, v.row.new)"
                        :key="k"
                        :class="{ w: s.changed && v.row.kind === 'change' }"
                        >{{ s.text }}</span
                      >
                    </span>
                  </div>
                </div>
              </template>
            </template>
            <p v-if="!rows.length" class="muted pad">{{ t('history.noDifferences') }}</p>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 55;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgb(0 0 0 / 45%);
}
.dialog {
  display: flex;
  flex-direction: column;
  width: min(1200px, 94vw);
  height: 86vh;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  outline: none;
}
header {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border);
}
h3 {
  margin: 0;
  font-size: 14px;
}
header label {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-muted);
}
select {
  max-width: 260px;
  padding: 3px var(--space-2);
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}
.spacer {
  flex: 1;
}
.body {
  display: flex;
  flex: 1;
  min-height: 0;
}
.files {
  display: flex;
  flex-direction: column;
  width: 280px;
  flex: none;
  border-right: 1px solid var(--border);
  overflow: auto;
}
.summary {
  padding: var(--space-2) var(--space-3);
  color: var(--text-muted);
}
.files ul {
  flex: 1;
  margin: 0;
  padding: 0;
  list-style: none;
}
.files li button {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  padding: 4px var(--space-3);
  background: none;
  border: 0;
  color: var(--text);
  text-align: left;
  cursor: pointer;
}
.files li button:hover,
.files li button.active {
  background: var(--bg-hover);
}
.path {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.st {
  width: 14px;
  text-align: center;
  font-family: var(--font-mono);
  font-size: 11px;
}
.st.added,
.add {
  color: var(--success, #3fb950);
}
.st.deleted,
.del {
  color: var(--danger, #f85149);
}
.st.modified,
.st.renamed {
  color: var(--warning, #d29922);
}
.restore {
  padding: var(--space-3);
  border-top: 1px solid var(--border);
}
.view {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
}
.toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--border);
}
.row {
  display: flex;
  align-items: center;
  gap: 4px;
}
.seg {
  display: inline-flex;
}
.seg button.on {
  background: var(--accent);
  color: var(--accent-contrast);
  border-color: var(--accent);
}
button {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px var(--space-2);
  background: var(--bg-hover);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  font: inherit;
}
button:disabled {
  opacity: 0.5;
  cursor: default;
}
.icon-btn {
  padding: 3px;
  background: none;
  border: 0;
  color: var(--text-muted);
}
.link {
  padding: 0 var(--space-2);
  background: none;
  border: 0;
  color: var(--accent);
  font-size: 11px;
}
.muted {
  margin: 0;
  padding: 0 var(--space-3);
  color: var(--text-muted);
}
.pad {
  padding: var(--space-3);
}
.error {
  margin: 0;
  padding: var(--space-2) var(--space-3);
  color: var(--danger, #f85149);
}
.code {
  flex: 1;
  overflow: auto;
  font-family: var(--font-mono);
  font-size: 12px;
}
.gap {
  padding: 2px var(--space-3);
  background: var(--bg-hover);
  color: var(--text-muted);
}
.hunk-bar {
  display: flex;
  justify-content: flex-end;
  background: var(--bg-hover);
}
.ln {
  display: flex;
  white-space: pre-wrap;
  word-break: break-word;
}
.ln .no {
  flex: none;
  width: 44px;
  padding-right: var(--space-2);
  text-align: right;
  color: var(--text-muted);
  user-select: none;
}
.ln .tx {
  flex: 1;
  min-width: 0;
}
.ln.add {
  background: rgb(63 185 80 / 15%);
}
.ln.rem {
  background: rgb(248 81 73 / 15%);
}
.ln.add .w {
  background: rgb(63 185 80 / 40%);
}
.ln.rem .w {
  background: rgb(248 81 73 / 40%);
}
.ln.empty {
  background: var(--bg-hover);
}
.split {
  display: grid;
  grid-template-columns: 1fr 1fr;
}
.split .half:first-child {
  border-right: 1px solid var(--border);
}
.binary {
  display: flex;
  gap: var(--space-4);
  padding: var(--space-3);
  overflow: auto;
}
.binary figure {
  flex: 1;
  margin: 0;
  text-align: center;
}
.binary img {
  max-width: 100%;
  max-height: 60vh;
  border: 1px solid var(--border);
}
</style>
