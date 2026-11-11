<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { CommitInfo } from '../history/api'
import { commitTitle, groupSessions } from '../history/model'
import { useHistoryStore } from '../stores/history'
import { useProjectStore } from '../stores/project'
import AppIcon from './AppIcon.vue'

const { t, locale } = useI18n()
const history = useHistoryStore()
const project = useProjectStore()

const message = ref('')
const amend = ref(false)
const changesOpen = ref(false)
const settingsOpen = ref(false)
const menuFor = ref<string | null>(null)

const sessions = computed(() => groupSessions(history.commits))
const staged = computed(() => history.status.entries.filter((e) => e.staged))
const unstaged = computed(() => history.status.entries.filter((e) => e.unstaged))

function dayLabel(day: string): string {
  const [y, m, d] = day.split('-').map(Number)
  return new Date(y, m - 1, d).toLocaleDateString(locale.value, {
    weekday: 'short',
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  })
}
const time = (c: CommitInfo) =>
  new Date(c.time * 1000).toLocaleTimeString(locale.value, { hour: '2-digit', minute: '2-digit' })

function onScroll(e: Event) {
  const el = e.target as HTMLElement
  if (el.scrollTop + el.clientHeight >= el.scrollHeight - 80) void history.loadMore()
}

async function doCommit() {
  if (!amend.value && !message.value.trim()) return
  const ok = await history.commitManual(message.value, amend.value)
  if (ok) {
    message.value = ''
    amend.value = false
  }
}

function toggleMenu(id: string) {
  menuFor.value = menuFor.value === id ? null : id
}
function act(fn: () => void) {
  menuFor.value = null
  fn()
}

const minutes = computed({
  get: () => history.config.minutes,
  set: (v: number) =>
    history.setConfig({ minutes: Math.min(120, Math.max(1, Math.round(v) || 5)) }),
})
</script>

<template>
  <div class="history">
    <div class="head">
      <h2>{{ t('history.title') }}</h2>
      <span class="head-actions">
        <button
          v-if="history.initialized"
          class="icon-btn"
          :title="t('history.settings')"
          :aria-label="t('history.settings')"
          :aria-pressed="settingsOpen"
          @click="settingsOpen = !settingsOpen"
        >
          <AppIcon name="cog-outline" :size="15" />
        </button>
        <button
          class="icon-btn"
          :title="t('history.refresh')"
          :aria-label="t('history.refresh')"
          @click="history.refresh()"
        >
          <AppIcon name="refresh" :size="15" />
        </button>
      </span>
    </div>

    <p v-if="!project.root" class="muted">{{ t('sidebar.empty') }}</p>

    <div v-else-if="!history.initialized && !history.declined" class="offer">
      <p>{{ t('history.offerText') }}</p>
      <button class="primary" :disabled="history.busy" @click="history.enable()">
        {{ t('history.enable') }}
      </button>
      <button @click="history.decline()">{{ t('history.notNow') }}</button>
    </div>

    <div v-else-if="!history.initialized" class="offer">
      <p class="muted">{{ t('history.disabled') }}</p>
      <button :disabled="history.busy" @click="history.enable()">{{ t('history.enable') }}</button>
    </div>

    <template v-else>
      <div class="toolbar">
        <button
          class="primary"
          :disabled="history.busy || history.merging"
          @click="history.checkpoint()"
        >
          <AppIcon name="content-save-outline" :size="14" /> {{ t('history.checkpoint') }}
        </button>
        <button
          :disabled="history.busy || history.merging"
          @click="history.nameDialog = { target: null }"
        >
          <AppIcon name="tag-outline" :size="14" /> {{ t('history.nameVersion') }}
        </button>
      </div>

      <p v-if="history.merging" class="warn">{{ t('history.mergeBlocked') }}</p>

      <div v-if="settingsOpen" class="box">
        <label class="row">
          <input
            type="checkbox"
            :checked="history.config.auto"
            @change="history.setConfig({ auto: ($event.target as HTMLInputElement).checked })"
          />
          {{ t('history.auto') }}
        </label>
        <label class="row">
          {{ t('history.autoEvery') }}
          <input
            v-model.number="minutes"
            class="num"
            type="number"
            min="1"
            max="120"
            :disabled="!history.config.auto"
          />
          {{ t('history.minutes') }}
        </label>
        <p class="muted small">
          {{ history.identity ? `${history.identity.name} <${history.identity.email}>` : '' }}
        </p>
      </div>

      <button class="section" :aria-expanded="changesOpen" @click="changesOpen = !changesOpen">
        <AppIcon :name="changesOpen ? 'chevron-down' : 'chevron-right'" :size="14" />
        {{ t('history.changes') }}
        <span class="badge">{{ history.status.entries.length }}</span>
      </button>
      <div v-if="changesOpen" class="changes">
        <p v-if="!history.status.entries.length" class="muted small">
          {{ t('history.noChanges') }}
        </p>
        <template v-else>
          <div v-if="staged.length" class="group-title">
            {{ t('history.staged') }}
          </div>
          <div v-for="e in staged" :key="'s' + e.path" class="file">
            <span :class="['st', e.status]">{{ e.status[0].toUpperCase() }}</span>
            <span class="path" :title="e.path">{{ e.path }}</span>
            <button
              class="icon-btn"
              :title="t('history.unstage')"
              :aria-label="t('history.unstage')"
              @click="history.stage([e.path], false)"
            >
              <AppIcon name="minus" :size="13" />
            </button>
          </div>
          <div v-if="unstaged.length" class="group-title">{{ t('history.unstaged') }}</div>
          <div v-for="e in unstaged" :key="'u' + e.path" class="file">
            <span :class="['st', e.status]">{{
              e.untracked ? 'U' : e.status[0].toUpperCase()
            }}</span>
            <span class="path" :title="e.path">{{ e.path }}</span>
            <button
              class="icon-btn"
              :title="t('history.stage')"
              :aria-label="t('history.stage')"
              @click="history.stage([e.path], true)"
            >
              <AppIcon name="plus" :size="13" />
            </button>
          </div>
          <div class="file-actions">
            <button
              class="link"
              @click="
                history.stage(
                  unstaged.map((e) => e.path),
                  true,
                )
              "
            >
              {{ t('history.stageAll') }}
            </button>
            <button class="link" @click="history.compareWorkdir()">
              {{ t('history.viewChanges') }}
            </button>
          </div>
        </template>
        <textarea
          v-model="message"
          rows="2"
          :placeholder="t('history.commitMessage')"
          :aria-label="t('history.commitMessage')"
        />
        <label class="row small">
          <input v-model="amend" type="checkbox" /> {{ t('history.amend') }}
        </label>
        <button
          :disabled="history.busy || (!amend && !message.trim()) || (!staged.length && !amend)"
          @click="doCommit"
        >
          <AppIcon name="source-commit" :size="14" /> {{ t('history.commit') }}
        </button>
      </div>

      <div class="filters">
        <label class="row small">
          <input v-model="history.filters.namedOnly" type="checkbox" />
          {{ t('history.filterNamed') }}
        </label>
        <input
          v-model="history.filters.path"
          type="search"
          :placeholder="t('history.filterPath')"
          :aria-label="t('history.filterPath')"
          list="history-files"
        />
        <datalist id="history-files">
          <option v-for="f in project.files" :key="f" :value="f" />
        </datalist>
        <input
          v-model="history.filters.author"
          type="search"
          :placeholder="t('history.filterAuthor')"
          :aria-label="t('history.filterAuthor')"
        />
      </div>

      <div v-if="history.dirty" class="current">
        <span>{{ t('history.unsaved', { n: history.status.entries.length }) }}</span>
        <button class="link" @click="history.compareWorkdir()">
          {{ t('history.viewChanges') }}
        </button>
      </div>

      <div class="timeline" role="list" @scroll="onScroll">
        <p v-if="!history.commits.length && !history.loading" class="muted">
          {{ t('history.empty') }}
        </p>
        <section v-for="s in sessions" :key="s.key" class="session">
          <div class="day">{{ dayLabel(s.day) }}</div>
          <div
            v-for="c in s.commits"
            :key="c.id"
            class="commit"
            :class="{ named: c.tags.length }"
            role="listitem"
          >
            <button class="main" :title="c.message" @click="history.compareCommit(c)">
              <span class="title">
                <AppIcon v-if="c.tags.length" name="tag-outline" :size="13" />
                {{ commitTitle(c) }}
              </span>
              <span class="meta">
                {{ time(c) }} · {{ c.author }} · {{ t('history.files', { n: c.filesChanged }) }}
              </span>
              <span v-if="c.tags[0]?.description" class="meta desc">{{
                c.tags[0].description
              }}</span>
            </button>
            <button
              class="icon-btn more"
              :aria-label="t('history.actions')"
              :title="t('history.actions')"
              @click="toggleMenu(c.id)"
            >
              <AppIcon name="chevron-down" :size="14" />
            </button>
            <ul v-if="menuFor === c.id" class="menu">
              <li>
                <button @click="act(() => history.compareCommit(c))">
                  {{ t('history.viewChanges') }}
                </button>
              </li>
              <li>
                <button @click="act(() => history.compareWithCurrent(c))">
                  {{ t('history.compareCurrent') }}
                </button>
              </li>
              <li v-if="!c.tags.length">
                <button @click="act(() => (history.nameDialog = { target: c.id }))">
                  {{ t('history.nameThis') }}
                </button>
              </li>
              <template v-for="tg in c.tags" :key="tg.name">
                <li>
                  <button @click="act(() => (history.nameDialog = { target: c.id, tag: tg }))">
                    {{ t('history.rename') }}: {{ tg.label }}
                  </button>
                </li>
                <li>
                  <button @click="act(() => history.removeVersion(tg.name))">
                    {{ t('history.removeLabel') }}
                  </button>
                </li>
              </template>
              <li>
                <button @click="act(() => history.compare(c, 'workdir'))">
                  {{ t('history.restoreProject') }}…
                </button>
              </li>
              <li>
                <button @click="act(() => history.exportZip(c))">
                  {{ t('history.exportZip') }}
                </button>
              </li>
              <li>
                <button @click="act(() => history.exportPdf(c))">
                  {{ t('history.exportPdf') }}
                </button>
              </li>
            </ul>
          </div>
        </section>
        <button
          v-if="history.hasMore"
          class="link more-btn"
          :disabled="history.loading"
          @click="history.loadMore()"
        >
          {{ t('history.loadMore') }}
        </button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.history {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  gap: var(--space-2);
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
h2 {
  margin: 0;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
}
.head-actions {
  display: inline-flex;
  gap: 2px;
}
.icon-btn {
  display: inline-flex;
  padding: 3px;
  background: none;
  border: 0;
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  cursor: pointer;
}
.icon-btn:hover,
.icon-btn[aria-pressed='true'] {
  background: var(--bg-hover);
  color: var(--text);
}
.muted {
  margin: 0;
  color: var(--text-muted);
}
.small {
  font-size: 11px;
}
.warn {
  margin: 0;
  padding: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  color: var(--text);
}
.offer {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.offer p {
  margin: 0;
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
.primary {
  background: var(--accent);
  color: var(--accent-contrast);
  border-color: var(--accent);
}
.toolbar {
  display: flex;
  gap: var(--space-2);
  flex-wrap: wrap;
}
.box,
.changes {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}
.row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.num {
  width: 56px;
}
input[type='number'],
input[type='search'],
textarea {
  padding: 4px var(--space-2);
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font: inherit;
}
textarea {
  resize: vertical;
}
.section {
  background: none;
  border: 0;
  padding: 2px 0;
  color: var(--text-muted);
  text-transform: uppercase;
  font-size: 11px;
  letter-spacing: 0.04em;
}
.badge {
  padding: 0 6px;
  border-radius: 8px;
  background: var(--bg-hover);
  color: var(--text);
}
.group-title {
  font-size: 11px;
  color: var(--text-muted);
}
.file {
  display: flex;
  align-items: center;
  gap: var(--space-2);
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
.st.added {
  color: var(--success, #3fb950);
}
.st.deleted,
.st.conflicted {
  color: var(--danger, #f85149);
}
.st.modified,
.st.renamed {
  color: var(--warning, #d29922);
}
.file-actions {
  display: flex;
  gap: var(--space-3);
}
.link {
  background: none;
  border: 0;
  padding: 0;
  color: var(--accent);
}
.filters {
  display: flex;
  flex-direction: column;
  gap: var(--space-1, 4px);
}
.current {
  display: flex;
  justify-content: space-between;
  gap: var(--space-2);
  padding: var(--space-2);
  background: var(--bg-hover);
  border-radius: var(--radius-sm);
}
.timeline {
  flex: 1;
  min-height: 120px;
  overflow: auto;
}
.day {
  position: sticky;
  top: 0;
  padding: 4px 0;
  background: var(--bg-panel);
  font-size: 11px;
  text-transform: uppercase;
  color: var(--text-muted);
}
.commit {
  position: relative;
  display: flex;
  align-items: flex-start;
  border-left: 2px solid var(--border);
  margin-left: 4px;
}
.commit.named {
  border-left-color: var(--accent);
  background: var(--bg-hover);
}
.main {
  flex: 1;
  min-width: 0;
  flex-direction: column;
  align-items: flex-start;
  gap: 0;
  padding: 4px var(--space-2);
  background: none;
  border: 0;
  text-align: left;
}
.main:hover {
  background: var(--bg-hover);
}
.title {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.named .title {
  font-weight: 600;
}
.meta {
  font-size: 11px;
  color: var(--text-muted);
}
.desc {
  white-space: normal;
}
.menu {
  position: absolute;
  right: 0;
  top: 100%;
  z-index: 20;
  margin: 0;
  padding: 4px;
  min-width: 200px;
  list-style: none;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow);
}
.menu button {
  width: 100%;
  background: none;
  border: 0;
}
.menu button:hover {
  background: var(--bg-hover);
}
.more-btn {
  margin: var(--space-2) 0;
}
</style>
