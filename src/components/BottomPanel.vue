<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useCompileStore } from '../stores/compile'
import ProblemsPanel from './ProblemsPanel.vue'
import RawLogPanel from './RawLogPanel.vue'

const { t } = useI18n()
const compile = useCompileStore()
const tab = ref<'problems' | 'log'>('problems')
</script>

<template>
  <div class="bottom">
    <div class="tabs" role="tablist">
      <button
        role="tab"
        :aria-selected="tab === 'problems'"
        :class="{ active: tab === 'problems' }"
        @click="tab = 'problems'"
      >
        {{ t('bottom.problems') }}
        <span v-if="compile.counts.errors" class="badge error">{{ compile.counts.errors }}</span>
        <span v-if="compile.counts.warnings" class="badge">{{ compile.counts.warnings }}</span>
      </button>
      <button
        role="tab"
        :aria-selected="tab === 'log'"
        :class="{ active: tab === 'log' }"
        @click="tab = 'log'"
      >
        {{ t('bottom.log') }}
      </button>
    </div>
    <div class="content" role="tabpanel">
      <ProblemsPanel v-if="tab === 'problems'" />
      <RawLogPanel v-else />
    </div>
  </div>
</template>

<style scoped>
.bottom {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
.tabs {
  display: flex;
  gap: var(--space-3);
  margin-bottom: var(--space-2);
}
.tabs button {
  background: none;
  border: 0;
  border-bottom: 2px solid transparent;
  padding: 0 0 2px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
  cursor: pointer;
}
.tabs button.active {
  color: var(--text);
  border-bottom-color: var(--accent);
}
.badge {
  margin-left: 4px;
  padding: 0 5px;
  border-radius: 8px;
  background: var(--bg-hover);
  color: var(--text);
}
.badge.error {
  background: var(--danger);
  color: var(--accent-contrast);
}
.content {
  flex: 1;
  min-height: 0;
  overflow: auto;
}
</style>
