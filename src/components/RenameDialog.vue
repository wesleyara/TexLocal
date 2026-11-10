<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { renameLabel } from '../editor/actions'
import { useRefsStore } from '../stores/refs'

const { t } = useI18n()
const refs = useRefsStore()
const value = ref('')
const input = ref<HTMLInputElement | null>(null)

watch(
  () => refs.renaming,
  async (name) => {
    if (name === null) return
    value.value = name
    await nextTick()
    input.value?.focus()
    input.value?.select()
  },
)

async function confirm() {
  const old = refs.renaming
  if (old === null) return
  if (value.value !== old && !(await renameLabel(old, value.value.trim()))) return
  refs.renaming = null
}
</script>

<template>
  <div v-if="refs.renaming !== null" class="overlay" @mousedown.self="refs.renaming = null">
    <form
      class="app-dialog"
      role="dialog"
      aria-modal="true"
      :aria-label="t('editor.rename.title')"
      @submit.prevent="confirm"
      @keydown.esc.stop="refs.renaming = null"
    >
      <h3>{{ t('editor.rename.title') }}</h3>
      <input ref="input" v-model="value" :aria-label="t('editor.rename.title')" />
      <div class="actions">
        <button type="button" @click="refs.renaming = null">{{ t('wizard.cancel') }}</button>
        <button type="submit" class="primary">{{ t('editor.rename.confirm') }}</button>
      </div>
    </form>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 15vh;
  background: rgb(0 0 0 / 40%);
}
.app-dialog {
  width: 380px;
  padding: var(--space-4);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
}
h3 {
  margin: 0 0 var(--space-3);
  font-size: 14px;
}
input {
  width: 100%;
  padding: 6px var(--space-2);
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  margin-top: var(--space-3);
}
button {
  padding: 5px var(--space-3);
  background: var(--bg-hover);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.primary {
  background: var(--accent);
  color: var(--accent-contrast);
  border-color: var(--accent);
}
</style>
