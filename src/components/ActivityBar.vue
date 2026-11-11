<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useUiStore } from '../stores/ui'
import AppIcon from './AppIcon.vue'

const { t } = useI18n()
const ui = useUiStore()

const items = [
  { id: 'files', key: 'activity.files', icon: 'file-tree' },
  { id: 'search', key: 'activity.search', icon: 'magnify' },
  { id: 'history', key: 'activity.history', icon: 'history' },
  { id: 'git', key: 'activity.git', icon: 'source-branch' },
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
        (i.id === 'files' || i.id === 'search' || i.id === 'history') &&
        ui.layout.sidebarVisible &&
        ui.sidebarPanel === i.id
      "
      @click="
        (i.id === 'files' || i.id === 'search' || i.id === 'history') && ui.showSidebarPanel(i.id)
      "
    >
      <AppIcon :name="i.icon" :size="22" />
    </button>
    <span class="spacer" />
    <button
      :title="t('activity.settings')"
      :aria-label="t('activity.settings')"
      @click="ui.settingsOpen = true"
    >
      <AppIcon name="cog-outline" :size="22" />
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
  display: flex;
  align-items: center;
  justify-content: center;
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
