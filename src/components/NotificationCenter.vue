<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useNotificationsStore } from '../stores/notifications'

const { t } = useI18n()
const n = useNotificationsStore()
const emit = defineEmits<{ close: [] }>()

function openTask(open: () => void) {
  open()
  emit('close')
}
</script>

<template>
  <section
    class="center"
    role="dialog"
    :aria-label="t('notify.center')"
    @keydown.esc="$emit('close')"
  >
    <h2>{{ t('notify.tasks') }}</h2>
    <p v-if="!n.tasks.length" class="muted">{{ t('notify.noTasks') }}</p>
    <ul>
      <li v-for="task in n.tasks" :key="task.id">
        <span>{{ task.title }}</span>
        <progress :value="task.progress ?? undefined" max="1" />
        <button v-if="task.open" @click="openTask(task.open)">{{ t('notify.open') }}</button>
        <button v-if="task.cancel" @click="n.cancelTask(task.id)">{{ t('notify.cancel') }}</button>
      </li>
    </ul>
    <h2>
      {{ t('notify.center') }}
      <button v-if="n.history.length" class="link" @click="n.clearHistory()">
        {{ t('notify.clear') }}
      </button>
    </h2>
    <p v-if="!n.history.length" class="muted">{{ t('notify.empty') }}</p>
    <ul>
      <li v-for="item in n.history" :key="item.id" :class="item.kind">{{ item.message }}</li>
    </ul>
  </section>
</template>

<style scoped>
.center {
  position: absolute;
  right: var(--space-2);
  bottom: 100%;
  z-index: 60;
  width: 340px;
  max-height: 360px;
  overflow: auto;
  padding: var(--space-3);
  background: var(--bg-elevated);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
}
h2 {
  display: flex;
  justify-content: space-between;
  margin: 0 0 var(--space-2);
  font-size: 12px;
  text-transform: uppercase;
  color: var(--text-muted);
}
ul {
  margin: 0 0 var(--space-3);
  padding: 0;
  list-style: none;
}
li {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}
li.error {
  color: var(--danger);
}
li.success {
  color: var(--success);
}
.muted {
  color: var(--text-muted);
  margin: 0 0 var(--space-3);
}
.link {
  background: none;
  border: 0;
  color: var(--accent);
  cursor: pointer;
}
progress {
  width: 100%;
}
</style>
