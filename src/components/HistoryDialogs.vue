<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useHistoryStore } from '../stores/history'

const { t } = useI18n()
const history = useHistoryStore()

const name = ref('')
const email = ref('')
const error = ref('')
const nameInput = ref<HTMLInputElement | null>(null)

watch(
  () => history.identityDialog,
  async (open) => {
    if (!open) return
    error.value = ''
    name.value = history.identity?.name ?? ''
    email.value = history.identity?.email ?? ''
    await nextTick()
    nameInput.value?.focus()
  },
)

async function saveIdentity() {
  error.value = (await history.saveIdentity(name.value, email.value)) ?? ''
}

const label = ref('')
const description = ref('')
const labelInput = ref<HTMLInputElement | null>(null)

watch(
  () => history.nameDialog,
  async (d) => {
    if (!d) return
    label.value = d.tag?.label ?? ''
    description.value = d.tag?.description ?? ''
    await nextTick()
    labelInput.value?.focus()
  },
)

async function saveName() {
  const d = history.nameDialog
  if (!d || !label.value.trim()) return
  history.nameDialog = null
  if (d.tag) await history.renameVersion(d.tag.name, label.value.trim(), description.value)
  else await history.nameVersion(label.value.trim(), description.value, d.target)
}
</script>

<template>
  <div v-if="history.identityDialog" class="overlay" @mousedown.self="history.cancelIdentity()">
    <form
      class="app-dialog"
      role="dialog"
      aria-modal="true"
      :aria-label="t('history.identity.title')"
      @submit.prevent="saveIdentity"
      @keydown.esc.stop="history.cancelIdentity()"
    >
      <h3>{{ t('history.identity.title') }}</h3>
      <p class="hint">{{ t('history.identity.hint') }}</p>
      <label>
        {{ t('history.identity.name') }}
        <input ref="nameInput" v-model="name" autocomplete="name" />
      </label>
      <label>
        {{ t('history.identity.email') }}
        <input v-model="email" type="email" autocomplete="email" />
      </label>
      <p v-if="error" class="error">{{ error }}</p>
      <div class="actions">
        <button type="button" @click="history.cancelIdentity()">{{ t('wizard.cancel') }}</button>
        <button type="submit" class="primary">{{ t('history.identity.save') }}</button>
      </div>
    </form>
  </div>

  <div v-if="history.nameDialog" class="overlay" @mousedown.self="history.nameDialog = null">
    <form
      class="app-dialog"
      role="dialog"
      aria-modal="true"
      :aria-label="t('history.nameVersion')"
      @submit.prevent="saveName"
      @keydown.esc.stop="history.nameDialog = null"
    >
      <h3>{{ history.nameDialog.tag ? t('history.rename') : t('history.nameVersion') }}</h3>
      <label>
        {{ t('history.versionName') }}
        <input ref="labelInput" v-model="label" :placeholder="t('history.versionPlaceholder')" />
      </label>
      <label>
        {{ t('history.versionDescription') }}
        <textarea v-model="description" rows="3" />
      </label>
      <div class="actions">
        <button type="button" @click="history.nameDialog = null">{{ t('wizard.cancel') }}</button>
        <button type="submit" class="primary" :disabled="!label.trim()">
          {{ t('history.save') }}
        </button>
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
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  width: 400px;
  padding: var(--space-4);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
}
h3 {
  margin: 0;
  font-size: 14px;
}
.hint {
  margin: 0;
  color: var(--text-muted);
}
.error {
  margin: 0;
  color: var(--danger, #f85149);
}
label {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
input,
textarea {
  padding: 6px var(--space-2);
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font: inherit;
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  margin-top: var(--space-2);
}
button {
  padding: 5px var(--space-3);
  background: var(--bg-hover);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
button:disabled {
  opacity: 0.5;
}
.primary {
  background: var(--accent);
  color: var(--accent-contrast);
  border-color: var(--accent);
}
</style>
