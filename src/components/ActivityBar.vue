<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useUiStore } from '../stores/ui'

const { t } = useI18n()
const ui = useUiStore()

const items = [
  { id: 'files', key: 'activity.files', glyph: '▤' },
  { id: 'search', key: 'activity.search', glyph: '⌕' },
  { id: 'history', key: 'activity.history', glyph: '↺' },
  { id: 'git', key: 'activity.git', glyph: '⑂' },
]
</script>

<template>
  <nav class="activity" :aria-label="t('app.name')">
    <button
      v-for="i in items"
      :key="i.id"
      :title="t(i.key)"
      :aria-label="t(i.key)"
      :aria-pressed="
        (i.id === 'files' || i.id === 'search') &&
        ui.layout.sidebarVisible &&
        ui.sidebarPanel === i.id
      "
      @click="(i.id === 'files' || i.id === 'search') && ui.showSidebarPanel(i.id)"
    >
      <span aria-hidden="true">{{ i.glyph }}</span>
    </button>
    <span class="spacer" />
    <button
      :title="t('activity.settings')"
      :aria-label="t('activity.settings')"
      @click="ui.settingsOpen = true"
    >
      <span aria-hidden="true">⚙</span>
    </button>
  </nav>
</template>

<style scoped>
.activity {
  display: flex;
  flex-direction: column;
  width: 44px;
  flex: none;
  background: var(--bg-panel);
  border-right: 1px solid var(--border);
}
button {
  height: 44px;
  background: none;
  border: 0;
  color: var(--text-muted);
  font-size: 18px;
  cursor: pointer;
}
button:hover,
button[aria-pressed='true'] {
  color: var(--text);
}
button[aria-pressed='true'] {
  box-shadow: inset 2px 0 var(--accent);
}
.spacer {
  flex: 1;
}
</style>
