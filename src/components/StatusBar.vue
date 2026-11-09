<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useNotificationsStore } from '../stores/notifications'
import NotificationCenter from './NotificationCenter.vue'

const { t } = useI18n()
const notifications = useNotificationsStore()
const centerOpen = ref(false)

// Valores reais chegam com os épicos de git (EP06/07), compilação (EP04) e editor (EP03).
const items = [
  { key: 'status.branch', value: '—' },
  { key: 'status.sync', value: '—' },
  { key: 'status.compile', value: '—' },
  { key: 'status.engine', value: '—' },
  { key: 'status.words', value: '0' },
  { key: 'status.cursor', value: '1:1' },
  { key: 'status.eol', value: 'LF' },
  { key: 'status.spell', value: '—' },
]
</script>

<template>
  <footer class="statusbar">
    <span v-for="i in items" :key="i.key" class="item" :title="t(i.key)">
      <span class="sr-only">{{ t(i.key) }}:</span>{{ i.value }}
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
