<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { wordCountFiles, type ProjectCount } from '../project/api'
import { useEditorStore } from '../stores/editor'
import { useNotificationsStore } from '../stores/notifications'
import { useProjectStore } from '../stores/project'
import NotificationCenter from './NotificationCenter.vue'

const { t } = useI18n()
const notifications = useNotificationsStore()
const editor = useEditorStore()
const project = useProjectStore()
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
const encoding = computed(() => (editor.activeTab?.encoding === 'latin1' ? 'Latin-1' : 'UTF-8'))
const spell = computed(() => {
  const lang = project.spellLanguage
  if (!lang) return '—'
  return project.dictionaries.includes(lang) ? lang : `${lang} ✖`
})

function toggleEol() {
  const tab = editor.activeTab
  if (tab) editor.setEol(tab.path, tab.eol === 'lf' ? 'crlf' : 'lf')
}

// Valores reais chegam com os épicos de git (EP06/07) e compilação (EP04).
const placeholders = [
  { key: 'status.branch', value: '—' },
  { key: 'status.sync', value: '—' },
  { key: 'status.compile', value: '—' },
  { key: 'status.engine', value: '—' },
]
</script>

<template>
  <footer class="statusbar">
    <span v-for="i in placeholders" :key="i.key" class="item" :title="t(i.key)">
      <span class="sr-only">{{ t(i.key) }}:</span>{{ i.value }}
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
    <span class="item" :title="t('status.encoding')">
      <span class="sr-only">{{ t('status.encoding') }}:</span>{{ encoding }}
    </span>
    <span class="item" :title="t('status.spell')">
      <span class="sr-only">{{ t('status.spell') }}:</span>{{ spell }}
    </span>
    <span class="spacer" />
    <button
      class="bell"
      :title="t('status.notifications')"
      :aria-label="t('status.notifications')"
      :aria-expanded="centerOpen"
      @click="centerOpen = !centerOpen"
    >
      🔔
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
