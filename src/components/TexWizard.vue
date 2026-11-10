<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useNotificationsStore } from '../stores/notifications'
import { useSettingsStore } from '../stores/settings'
import { useTexInstallStore } from '../stores/texInstall'
import { useUiStore } from '../stores/ui'
import {
  detectTex,
  distributionName,
  formatBytes,
  formatProfileSize,
  requiredBytes,
  texDiskInfo,
  texProfiles,
  type DiskInfo,
  type Distribution,
  type ProfileId,
  type ProfileInfo,
} from '../tex/api'

const { t } = useI18n()
const ui = useUiStore()
const settings = useSettingsStore()
const notifications = useNotificationsStore()
const install = useTexInstallStore()

type Step = 'welcome' | 'detect' | 'profile' | 'location' | 'install' | 'done'
const MANAGED = '__managed__'

const step = ref<Step>('welcome')
const detecting = ref(false)
const found = ref<Distribution[]>([])
const selected = ref<string>(MANAGED)
const profiles = ref<ProfileInfo[]>([])
const profile = ref<ProfileId>('recommended')
const disk = ref<DiskInfo>({ path: '', freeBytes: null })

const enough = (p: ProfileInfo) =>
  disk.value.freeBytes === null || disk.value.freeBytes >= requiredBytes(p.approxSizeMb)
const current = computed(() => profiles.value.find((p) => p.id === profile.value))
const canInstall = computed(() => (current.value ? enough(current.value) : false))

async function detect() {
  detecting.value = true
  try {
    found.value = await detectTex()
    selected.value = found.value[0]?.binDir ?? MANAGED
  } catch {
    found.value = []
    selected.value = MANAGED
    notifications.notify('error', t('wizard.detectFailed'))
  } finally {
    detecting.value = false
  }
}

watch(
  () => ui.wizardOpen,
  (open) => {
    if (!open) return
    if (install.status === 'running') {
      step.value = 'install'
      return
    }
    step.value = 'welcome'
    found.value = []
    selected.value = MANAGED
  },
)

watch(
  () => install.status,
  (s) => {
    if (s === 'done' && ui.wizardOpen) step.value = 'done'
  },
)

async function goDetect() {
  step.value = 'detect'
  await detect()
}

async function chooseSource() {
  if (selected.value !== MANAGED) {
    const ok = await settings.update({ texSource: 'system', texBinDir: selected.value })
    if (ok) step.value = 'done'
    else notifications.notify('error', t('settings.saveFailed'))
    return
  }
  const [p, d] = await Promise.all([texProfiles(), texDiskInfo()])
  profiles.value = p
  disk.value = d
  if (current.value && !enough(current.value)) {
    profile.value = p.find(enough)?.id ?? 'minimal'
  }
  step.value = 'profile'
}

async function startInstall() {
  step.value = 'install'
  await install.start(profile.value)
}

function retry() {
  install.reset()
  step.value = 'profile'
}

async function skip() {
  await settings.update({ texSource: 'skipped' })
  ui.wizardOpen = false
}

function close() {
  ui.wizardOpen = false
}

const percent = computed(() =>
  install.event?.progress == null ? null : Math.round(install.event.progress * 100),
)
</script>

<template>
  <div v-if="ui.wizardOpen" class="overlay" @mousedown.self="close">
    <div class="app-dialog" role="dialog" aria-modal="true" :aria-label="t('wizard.title')">
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
        <template v-else>
          <p>{{ found.length ? t('wizard.foundIntro') : t('wizard.noneFound') }}</p>
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
          <label class="option">
            <input v-model="selected" type="radio" name="dist" :value="MANAGED" />
            <span>
              <strong>{{ t('wizard.installManaged') }}</strong>
              <small>{{ t('wizard.installManagedHint') }}</small>
            </span>
          </label>
        </template>
        <footer>
          <button @click="skip">{{ t('wizard.skip') }}</button>
          <button :disabled="detecting" @click="detect">{{ t('wizard.detectAgain') }}</button>
          <button class="primary" :disabled="detecting" @click="chooseSource">
            {{ t('wizard.next') }}
          </button>
        </footer>
      </section>

      <section v-else-if="step === 'profile'" class="body">
        <p>{{ t('wizard.profileIntro') }}</p>
        <label v-for="p in profiles" :key="p.id" class="option" :class="{ disabled: !enough(p) }">
          <input
            v-model="profile"
            type="radio"
            name="profile"
            :value="p.id"
            :disabled="!enough(p)"
          />
          <span>
            <strong
              >{{ t(`wizard.profile.${p.id}.name`) }} · ~{{
                formatProfileSize(p.approxSizeMb)
              }}</strong
            >
            <small>{{ t(`wizard.profile.${p.id}.desc`) }}</small>
            <small v-if="!enough(p)" class="warn">{{ t('wizard.noSpace') }}</small>
          </span>
        </label>
        <footer>
          <button @click="step = 'detect'">{{ t('wizard.back') }}</button>
          <button class="primary" :disabled="!canInstall" @click="step = 'location'">
            {{ t('wizard.next') }}
          </button>
        </footer>
      </section>

      <section v-else-if="step === 'location'" class="body">
        <p>{{ t('wizard.locationIntro') }}</p>
        <p class="path">{{ disk.path }}</p>
        <p class="muted">
          <span v-if="disk.freeBytes !== null">
            {{ t('wizard.freeSpace', { free: formatBytes(disk.freeBytes) }) }} ·
          </span>
          <span v-if="current">
            {{ t('wizard.requires', { need: formatBytes(requiredBytes(current.approxSizeMb)) }) }}
          </span>
        </p>
        <footer>
          <button @click="step = 'profile'">{{ t('wizard.back') }}</button>
          <button class="primary" :disabled="!canInstall" @click="startInstall">
            {{ t('wizard.install') }}
          </button>
        </footer>
      </section>

      <section v-else-if="step === 'install'" class="body">
        <template v-if="install.status === 'error' && install.error">
          <p class="warn" role="alert">{{ t(install.error.i18nKey) }}</p>
          <details v-if="install.error.details">
            <summary>{{ t('wizard.details') }}</summary>
            <pre>{{ install.error.details }}</pre>
          </details>
          <footer>
            <button @click="skip">{{ t('wizard.skip') }}</button>
            <button class="primary" @click="retry">{{ t('wizard.retry') }}</button>
          </footer>
        </template>
        <template v-else>
          <p>{{ t(`wizard.phase.${install.event?.phase ?? 'preflight'}`) }}</p>
          <progress :value="install.event?.progress ?? undefined" max="1" />
          <p class="muted">
            <span v-if="percent !== null">{{ percent }}% · </span>{{ install.event?.message }}
          </p>
          <p class="muted">{{ t('wizard.backgroundHint') }}</p>
          <footer>
            <button @click="install.cancel()">{{ t('wizard.cancel') }}</button>
            <button class="primary" @click="close">{{ t('wizard.background') }}</button>
          </footer>
        </template>
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
.app-dialog {
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
.option.disabled {
  opacity: 0.55;
  cursor: default;
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
.path {
  padding: var(--space-2);
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
  overflow-wrap: anywhere;
}
progress {
  width: 100%;
}
pre {
  max-height: 200px;
  overflow: auto;
  font-size: 11px;
  white-space: pre-wrap;
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
