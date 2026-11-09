<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useNotificationsStore } from '../stores/notifications'
import { useSettingsStore } from '../stores/settings'
import { useUiStore } from '../stores/ui'
import { detectTex, distributionName, type Distribution } from '../tex/api'

const { t } = useI18n()
const ui = useUiStore()
const settings = useSettingsStore()
const notifications = useNotificationsStore()

type Step = 'welcome' | 'detect' | 'done'
const step = ref<Step>('welcome')
const detecting = ref(false)
const found = ref<Distribution[]>([])
const selected = ref<string | null>(null)

async function detect() {
  detecting.value = true
  try {
    found.value = await detectTex()
    selected.value = found.value[0]?.binDir ?? null
  } catch {
    found.value = []
    notifications.notify('error', t('wizard.detectFailed'))
  } finally {
    detecting.value = false
  }
}

watch(
  () => ui.wizardOpen,
  (open) => {
    if (open) {
      step.value = 'welcome'
      found.value = []
      selected.value = null
    }
  },
)

async function goDetect() {
  step.value = 'detect'
  await detect()
}

async function finish() {
  const ok = await settings.update({ texSource: 'system', texBinDir: selected.value ?? undefined })
  if (ok) step.value = 'done'
  else notifications.notify('error', t('settings.saveFailed'))
}

async function skip() {
  await settings.update({ texSource: 'skipped' })
  ui.wizardOpen = false
}

function close() {
  ui.wizardOpen = false
}
</script>

<template>
  <div v-if="ui.wizardOpen" class="overlay" @mousedown.self="close">
    <div class="dialog" role="dialog" aria-modal="true" :aria-label="t('wizard.title')">
      <header>
        <h1>{{ t('wizard.title') }}</h1>
        <button :aria-label="t('settings.close')" @click="close">×</button>
      </header>

      <section v-if="step === 'welcome'" class="body">
        <p>{{ t('wizard.welcome') }}</p>
        <footer>
          <button @click="skip">{{ t('wizard.skip') }}</button>
          <button class="primary" @click="goDetect">{{ t('wizard.start') }}</button>
        </footer>
      </section>

      <section v-else-if="step === 'detect'" class="body">
        <p v-if="detecting">{{ t('wizard.detecting') }}</p>
        <template v-else-if="found.length">
          <p>{{ t('wizard.foundIntro') }}</p>
          <label v-for="d in found" :key="d.binDir" class="option">
            <input v-model="selected" type="radio" name="dist" :value="d.binDir" />
            <span>
              <strong>{{ distributionName(d) }}</strong>
              <small>{{ d.binDir }}</small>
              <small>{{ d.engines.join(', ') }}</small>
              <small v-if="!d.hasLatexmk" class="warn">{{ t('wizard.noLatexmk') }}</small>
              <small v-if="!d.hasTlmgr" class="warn">{{ t('wizard.noTlmgr') }}</small>
            </span>
          </label>
        </template>
        <template v-else>
          <p>{{ t('wizard.noneFound') }}</p>
          <p class="muted">{{ t('wizard.managedUnavailable') }}</p>
        </template>
        <footer>
          <button @click="skip">{{ t('wizard.skip') }}</button>
          <button :disabled="detecting" @click="detect">{{ t('wizard.detectAgain') }}</button>
          <button class="primary" :disabled="!selected" @click="finish">
            {{ t('wizard.useSelected') }}
          </button>
        </footer>
      </section>

      <section v-else class="body">
        <p>{{ t('wizard.done') }}</p>
        <footer>
          <button class="primary" @click="close">{{ t('wizard.close') }}</button>
        </footer>
      </section>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 75;
  display: flex;
  justify-content: center;
  align-items: center;
  background: rgb(0 0 0 / 45%);
}
.dialog {
  width: min(560px, 92vw);
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
}
header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
}
h1 {
  margin: 0;
  font-size: 15px;
}
header button {
  background: none;
  border: 0;
  font-size: 20px;
  cursor: pointer;
}
.body {
  padding: var(--space-4);
  overflow: auto;
}
.option {
  display: flex;
  gap: var(--space-3);
  align-items: flex-start;
  padding: var(--space-2);
  margin-bottom: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.option span {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
small {
  color: var(--text-muted);
  overflow-wrap: anywhere;
}
.warn {
  color: var(--danger);
}
.muted {
  color: var(--text-muted);
}
footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  margin-top: var(--space-4);
}
footer button {
  padding: 6px var(--space-3);
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
footer button.primary {
  background: var(--accent);
  color: var(--accent-contrast);
  border-color: var(--accent);
}
footer button:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
