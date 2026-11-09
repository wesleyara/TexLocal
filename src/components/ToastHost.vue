<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useNotificationsStore } from '../stores/notifications'

const { t } = useI18n()
const n = useNotificationsStore()
</script>

<template>
  <div class="toasts" aria-live="polite">
    <div v-for="toast in n.toasts" :key="toast.id" class="toast" :class="toast.kind" role="status">
      <span>{{ toast.message }}</span>
      <button :aria-label="t('notify.dismiss')" @click="n.dismissToast(toast.id)">×</button>
    </div>
  </div>
</template>

<style scoped>
.toasts {
  position: fixed;
  right: var(--space-4);
  bottom: 40px;
  z-index: 80;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.toast {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  max-width: 360px;
  padding: var(--space-2) var(--space-3);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-left: 3px solid var(--accent);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
}
.toast.error {
  border-left-color: var(--danger);
}
.toast.success {
  border-left-color: var(--success);
}
button {
  margin-left: auto;
  background: none;
  border: 0;
  cursor: pointer;
  color: var(--text-muted);
  font-size: 16px;
}
</style>
