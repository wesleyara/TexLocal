<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { wordCountFiles, type Encoding, type ProjectCount } from '../project/api'
import { useCompileStore } from '../stores/compile'
import { useEditorStore } from '../stores/editor'
import { useNotificationsStore } from '../stores/notifications'
import { useProjectStore } from '../stores/project'
import { useSettingsStore } from '../stores/settings'
import { useUiStore } from '../stores/ui'
import { zoomFactor } from '../settings/zoom'
import AppIcon from './AppIcon.vue'
import NotificationCenter from './NotificationCenter.vue'

const { t } = useI18n()
const notifications = useNotificationsStore()
const editor = useEditorStore()
const project = useProjectStore()
const compile = useCompileStore()
const ui = useUiStore()
const settings = useSettingsStore()
const zoomLabel = computed(() => `${Math.round(zoomFactor(settings.effective.zoomLevel) * 100)}%`)
const centerOpen = ref(false)
const detailsOpen = ref(false)
const count = ref<ProjectCount | null>(null)

// Contagem do documento (arquivos da árvore a partir do principal), recalculada ao salvar/indexar.
let timer: ReturnType<typeof setTimeout> | undefined
watch(
  () => [project.view, editor.tabs.map((x) => x.dirty).join()],
  () => {
    clearTimeout(timer)
    timer = setTimeout(async () => {
      const paths = project.view.files.map((f) => f.path)
      count.value = paths.length ? await wordCountFiles(paths) : null
    }, 1200)
  },
  { deep: false },
)

const totalWords = computed(() =>
  count.value ? count.value.total.words + count.value.total.headers : 0,
)
const words = computed(() => {
  const sel = editor.cursor.selectionWords
  return sel !== null ? `${sel} / ${totalWords.value}` : String(totalWords.value)
})
const cursor = computed(() => `${editor.cursor.line}:${editor.cursor.col}`)
const eol = computed(() => (editor.activeTab?.eol ?? 'lf').toUpperCase())
const ENCODINGS: { id: Encoding; label: string }[] = [
  { id: 'utf8', label: 'UTF-8' },
  { id: 'latin1', label: 'ISO-8859-1 (Latin-1)' },
  { id: 'iso885915', label: 'ISO-8859-15 (Latin-9)' },
  { id: 'windows1252', label: 'Windows-1252' },
]
const encoding = computed(
  () => ENCODINGS.find((e) => e.id === editor.activeTab?.encoding)?.label.split(' ')[0] ?? 'UTF-8',
)
const encMenuOpen = ref(false)

function pickEncoding(mode: 'reopen' | 'save', id: Encoding) {
  const tab = editor.activeTab
  encMenuOpen.value = false
  if (!tab) return
  if (mode === 'reopen') void editor.reopenWithEncoding(tab.path, id)
  else editor.saveWithEncoding(tab.path, id)
}
const spell = computed(() => {
  const lang = project.spellLanguage
  if (!lang) return '—'
  return project.dictionaries.includes(lang) ? lang : `${lang} ✖`
})

function toggleEol() {
  const tab = editor.activeTab
  if (tab) editor.setEol(tab.path, tab.eol === 'lf' ? 'crlf' : 'lf')
}

// Valores reais de git chegam com os épicos EP06/07.
const placeholders = [
  { key: 'status.branch', value: '—' },
  { key: 'status.sync', value: '—' },
]

const compileLabel = computed(() => t(`compile.state.${compile.state}`))
const compileIcon = computed(
  () =>
    ({
      idle: 'checkbox-blank-circle-outline',
      running: 'sync',
      success: 'check-circle',
      failed: 'alert-circle',
      cancelled: 'stop-circle-outline',
      timedOut: 'timer-outline',
      error: 'alert-circle',
    })[compile.state],
)

function toggleProblems() {
  ui.layout.bottomVisible = !ui.layout.bottomVisible
}
</script>

<template>
  <footer class="statusbar">
    <span v-for="i in placeholders" :key="i.key" class="item" :title="t(i.key)">
      <span class="sr-only">{{ t(i.key) }}:</span>{{ i.value }}
    </span>
    <button
      class="item btn compile"
      :class="compile.state"
      :title="t('status.compile')"
      :aria-label="`${t('status.compile')}: ${compileLabel}`"
      @click="toggleProblems"
    >
      <AppIcon :name="compileIcon" :size="14" :class="{ spin: compile.state === 'running' }" />
      {{ compileLabel }}
      <template v-if="compile.counts.errors || compile.counts.warnings">
        · <AppIcon name="alert-circle" :size="13" /> {{ compile.counts.errors }} ·
        <AppIcon name="alert" :size="13" /> {{ compile.counts.warnings }}
      </template>
    </button>
    <button
      class="item btn"
      :title="t('status.autoCompile')"
      role="switch"
      :aria-checked="compile.auto"
      :disabled="!compile.manifest"
      @click="compile.toggleAuto()"
    >
      {{ t('status.auto') }}: {{ compile.auto ? t('status.on') : t('status.off') }}
    </button>
    <span v-if="compile.engine" class="item" :title="t('status.engine')">
      <span class="sr-only">{{ t('status.engine') }}:</span>{{ compile.engine }}
    </span>
    <button
      class="item btn"
      :title="t('status.words')"
      :aria-expanded="detailsOpen"
      @click="detailsOpen = !detailsOpen"
    >
      <span class="sr-only">{{ t('status.words') }}:</span>{{ words }}
    </button>
    <span class="item" :title="t('status.cursor')">
      <span class="sr-only">{{ t('status.cursor') }}:</span>{{ cursor }}
    </span>
    <button
      class="item btn"
      :title="t('status.eolChange')"
      :disabled="!editor.activeTab"
      @click="toggleEol"
    >
      <span class="sr-only">{{ t('status.eol') }}:</span>{{ eol }}
    </button>
    <span class="anchor">
      <button
        class="item btn"
        :title="t('status.encodingChange')"
        :disabled="!editor.activeTab"
        :aria-expanded="encMenuOpen"
        aria-haspopup="menu"
        @click="encMenuOpen = !encMenuOpen"
      >
        <span class="sr-only">{{ t('status.encoding') }}:</span>{{ encoding }}
      </button>
      <div
        v-if="encMenuOpen && editor.activeTab"
        class="encmenu"
        role="menu"
        @keydown.esc="encMenuOpen = false"
      >
        <div v-for="mode in ['reopen', 'save'] as const" :key="mode" class="group">
          <p class="head">
            {{ t(mode === 'reopen' ? 'status.encodingReopen' : 'status.encodingSave') }}
          </p>
          <button
            v-for="e in ENCODINGS"
            :key="e.id"
            role="menuitem"
            :disabled="mode === 'reopen' && editor.activeTab.dirty"
            :class="{ current: e.id === editor.activeTab.encoding }"
            @click="pickEncoding(mode, e.id)"
          >
            {{ e.label }}
          </button>
        </div>
      </div>
    </span>
    <span class="item" :title="t('status.spell')">
      <span class="sr-only">{{ t('status.spell') }}:</span>{{ spell }}
    </span>
    <span class="spacer" />
    <button
      class="item btn"
      :title="t('status.zoomReset')"
      :aria-label="`${t('status.zoom')}: ${zoomLabel}`"
      @click="settings.update({ zoomLevel: 0 })"
    >
      {{ zoomLabel }}
    </button>
    <button
      v-for="task in notifications.tasks"
      :key="task.id"
      class="item btn task"
      :title="t('notify.openTask', { title: task.title })"
      @click="task.open ? task.open() : (centerOpen = true)"
    >
      <AppIcon name="download" :size="13" /> {{ task.title }}
      <template v-if="task.progress !== null"> · {{ Math.round(task.progress * 100) }}%</template>
    </button>
    <button
      class="bell"
      :title="t('status.notifications')"
      :aria-label="t('status.notifications')"
      :aria-expanded="centerOpen"
      @click="centerOpen = !centerOpen"
    >
      <AppIcon
        :name="notifications.tasks.length ? 'bell-badge-outline' : 'bell-outline'"
        :size="15"
      />
      <span v-if="notifications.tasks.length" class="badge">{{ notifications.tasks.length }}</span>
    </button>
    <NotificationCenter v-if="centerOpen" @close="centerOpen = false" />
    <div
      v-if="detailsOpen && count"
      class="details"
      role="dialog"
      :aria-label="t('status.wordsDetail')"
    >
      <p class="sum">
        {{
          t('status.wordsBreakdown', {
            words: count.total.words,
            headers: count.total.headers,
            inline: count.total.inlineMath,
            display: count.total.displayMath,
          })
        }}
      </p>
      <ul>
        <li v-for="(s, i) in count.sections" :key="i">
          <span :style="{ paddingLeft: Math.max(s.level, 0) * 10 + 'px' }">
            {{ s.title || t('status.beforeFirstSection') }}
          </span>
          <span>{{ s.words }}</span>
        </li>
      </ul>
    </div>
  </footer>
</template>

<style scoped>
.statusbar {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-4);
  height: 24px;
  padding: 0 var(--space-3);
  flex: none;
  background: var(--bg-panel);
  border-top: 1px solid var(--border);
  color: var(--text-muted);
  font-size: 12px;
}
.spacer {
  flex: 1;
}
.btn {
  background: none;
  border: 0;
  padding: 0;
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.btn:disabled {
  cursor: default;
}
.compile {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.spin {
  animation: spin 1.2s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
.task {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--accent);
}
.compile.failed,
.compile.error,
.compile.timedOut {
  color: var(--danger);
}
.anchor {
  position: relative;
  display: inline-flex;
}
.encmenu {
  position: absolute;
  bottom: calc(100% + 6px);
  left: 50%;
  transform: translateX(-50%);
  z-index: 30;
  display: flex;
  gap: var(--space-3);
  padding: var(--space-2);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  color: var(--text);
}
.encmenu .group {
  display: flex;
  flex-direction: column;
}
.encmenu .head {
  margin: 0 0 var(--space-1);
  padding: 0 var(--space-2);
  color: var(--text-muted);
  font-size: 11px;
}
.encmenu button {
  background: none;
  border: 0;
  padding: 3px var(--space-2);
  border-radius: var(--radius);
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.encmenu button:hover:not(:disabled) {
  background: var(--bg-hover);
}
.encmenu button:disabled {
  color: var(--text-muted);
  cursor: default;
}
.encmenu button.current {
  color: var(--accent);
}
.details {
  position: absolute;
  bottom: 26px;
  left: var(--space-3);
  z-index: 30;
  width: 320px;
  max-height: 320px;
  overflow: auto;
  padding: var(--space-3);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  color: var(--text);
}
.details .sum {
  margin: 0 0 var(--space-2);
}
.details ul {
  margin: 0;
  padding: 0;
  list-style: none;
}
.details li {
  display: flex;
  justify-content: space-between;
  padding: 1px 0;
}
.bell {
  position: relative;
  background: none;
  border: 0;
  cursor: pointer;
  font-size: 12px;
}
.badge {
  margin-left: 2px;
  padding: 0 5px;
  border-radius: 8px;
  background: var(--accent);
  color: var(--accent-contrast);
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
}
</style>
