<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  allCommands,
  commandTitle,
  effectiveBinding,
  eventToBinding,
  findConflicts,
  formatBinding,
} from '../commands/registry'
import {
  validateSettings,
  type AutoInstall,
  type LocaleSetting,
  type SnippetOrder,
  type ThemeSetting,
} from '../settings/schema'
import { useNotificationsStore } from '../stores/notifications'
import { useSettingsStore } from '../stores/settings'
import { useSnippetsStore } from '../stores/snippets'
import { useUiStore } from '../stores/ui'
import ResourcesPanel from './ResourcesPanel.vue'

const { t } = useI18n()
const ui = useUiStore()
const settings = useSettingsStore()
const notifications = useNotificationsStore()
const snippets = useSnippetsStore()

function toggleCategory(category: string, enabled: boolean) {
  const off = new Set(settings.effective.snippetsDisabled)
  if (enabled) off.delete(category)
  else off.add(category)
  void save({ snippetsDisabled: [...off] })
}

const search = ref('')
const capturing = ref<string | null>(null)
const jsonText = ref('')
const jsonError = ref('')
const showJson = ref(false)

watch(
  () => ui.settingsOpen,
  (open) => {
    if (open) {
      search.value = ''
      capturing.value = null
      showJson.value = false
      jsonText.value = JSON.stringify(settings.global, null, 2)
    }
  },
)

const matches = (label: string) => label.toLowerCase().includes(search.value.trim().toLowerCase())

const showTheme = computed(() => matches(t('settings.theme')))
const showLanguage = computed(() => matches(t('settings.language')))

function titleOf(id: string): string {
  const x = allCommands().find((c) => c.id === id)
  return x ? commandTitle(x, t) : id
}

const shortcutRows = computed(() => {
  const overrides = settings.effective.keybindings
  const conflicts = findConflicts(allCommands(), overrides)
  return allCommands()
    .map((c) => {
      const b = effectiveBinding(c, overrides)
      const group = conflicts.find((g) => g.includes(c.id))
      return {
        id: c.id,
        title: commandTitle(c, t),
        shortcut: b ? formatBinding(b) : '—',
        overridden: c.id in overrides,
        conflictWith: group
          ? group
              .filter((id) => id !== c.id)
              .map((id) => titleOf(id))
              .join(', ')
          : '',
      }
    })
    .filter((r) => matches(r.title) || matches(t('settings.shortcuts')))
})

async function save(patch: Parameters<typeof settings.update>[0]) {
  const ok = await settings.update(patch)
  if (!ok) notifications.notify('error', t('settings.saveFailed'))
}

function onCaptureKey(e: KeyboardEvent) {
  if (!capturing.value) return
  e.preventDefault()
  e.stopPropagation()
  if (e.key === 'Escape') {
    capturing.value = null
    return
  }
  const b = eventToBinding(e)
  if (!b) return
  const id = capturing.value
  capturing.value = null
  void save({ keybindings: { ...settings.global.keybindings, [id]: b } })
}

function resetShortcut(id: string) {
  const next = { ...settings.global.keybindings }
  delete next[id]
  void save({ keybindings: next })
}

async function applyJson() {
  try {
    const parsed = validateSettings(JSON.parse(jsonText.value))
    jsonError.value = ''
    await save(parsed)
    notifications.notify('success', t('settings.saved'))
  } catch (err) {
    jsonError.value = t('settings.jsonInvalid', { error: (err as Error).message })
  }
}

function close() {
  ui.settingsOpen = false
}
</script>

<template>
  <div v-if="ui.settingsOpen" class="overlay" @mousedown.self="close">
    <div
      class="app-dialog"
      role="dialog"
      aria-modal="true"
      :aria-label="t('settings.title')"
      @keydown.capture="onCaptureKey"
      @keydown.esc="close"
    >
      <header>
        <h1>{{ t('settings.title') }}</h1>
        <button :aria-label="t('settings.close')" @click="close">×</button>
      </header>
      <input v-model="search" type="search" :placeholder="t('settings.search')" />

      <div class="body">
        <label v-if="showTheme" class="row">
          <span>{{ t('settings.theme') }}</span>
          <select
            :value="settings.effective.theme"
            @change="save({ theme: ($event.target as HTMLSelectElement).value as ThemeSetting })"
          >
            <option value="system">{{ t('settings.themeSystem') }}</option>
            <option value="dark">{{ t('settings.themeDark') }}</option>
            <option value="light">{{ t('settings.themeLight') }}</option>
          </select>
        </label>

        <label v-if="showLanguage" class="row">
          <span>{{ t('settings.language') }}</span>
          <select
            :value="settings.effective.locale"
            @change="save({ locale: ($event.target as HTMLSelectElement).value as LocaleSetting })"
          >
            <option value="auto">{{ t('settings.languageAuto') }}</option>
            <option value="pt-BR">Português (Brasil)</option>
            <option value="en">English</option>
          </select>
        </label>

        <label class="row">
          <span>
            {{ t('settings.texAutoInstall') }}
            <small class="hint">{{ t('settings.texAutoInstallHint') }}</small>
          </span>
          <select
            :value="settings.effective.texAutoInstall"
            @change="
              save({ texAutoInstall: ($event.target as HTMLSelectElement).value as AutoInstall })
            "
          >
            <option value="ask">{{ t('settings.autoInstallAsk') }}</option>
            <option value="auto">{{ t('settings.autoInstallAuto') }}</option>
            <option value="never">{{ t('settings.autoInstallNever') }}</option>
          </select>
        </label>

        <label class="row">
          <span>
            {{ t('settings.texMirror') }}
            <small class="hint">{{ t('settings.texMirrorHint') }}</small>
          </span>
          <input
            type="url"
            class="mirror"
            :value="settings.effective.texMirror ?? ''"
            placeholder="https://mirror.ctan.org/systems/texlive/tlnet"
            @change="save({ texMirror: ($event.target as HTMLInputElement).value.trim() })"
          />
        </label>
        <button class="row-btn" @click="ui.wizardOpen = true">{{ t('settings.texWizard') }}</button>

        <h2>{{ t('snippets.settingsTitle') }}</h2>
        <label class="row">
          <span>{{ t('snippets.order') }}</span>
          <select
            :value="settings.effective.snippetsOrder"
            @change="
              save({ snippetsOrder: ($event.target as HTMLSelectElement).value as SnippetOrder })
            "
          >
            <option value="before">{{ t('snippets.orderBefore') }}</option>
            <option value="after">{{ t('snippets.orderAfter') }}</option>
          </select>
        </label>
        <p class="hint">{{ t('snippets.categoriesHint') }}</p>
        <label v-for="c in snippets.categories" :key="c" class="row cat">
          <span>{{ t(`snippets.category.${c}`) }}</span>
          <input
            type="checkbox"
            :checked="!snippets.disabled.has(c)"
            @change="toggleCategory(c, ($event.target as HTMLInputElement).checked)"
          />
        </label>
        <button class="row-btn" @click="ui.snippetsOpen = true">
          {{ t('cmd.manageSnippets') }}
        </button>

        <ResourcesPanel />

        <h2>{{ t('settings.shortcuts') }}</h2>
        <ul class="shortcuts">
          <li v-for="r in shortcutRows" :key="r.id">
            <span>{{ r.title }}</span>
            <kbd>{{ capturing === r.id ? t('settings.shortcutPress') : r.shortcut }}</kbd>
            <button @click="capturing = r.id">{{ t('settings.shortcutChange') }}</button>
            <button v-if="r.overridden" @click="resetShortcut(r.id)">
              {{ t('settings.shortcutReset') }}
            </button>
            <small v-if="r.conflictWith" class="conflict" role="alert">
              {{ t('settings.shortcutConflict', { with: r.conflictWith }) }}
            </small>
          </li>
        </ul>

        <button class="link" @click="showJson = !showJson">{{ t('settings.json') }}</button>
        <div v-if="showJson">
          <textarea v-model="jsonText" rows="10" spellcheck="false" />
          <p v-if="jsonError" class="conflict" role="alert">{{ jsonError }}</p>
          <button @click="applyJson">{{ t('settings.jsonApply') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  justify-content: center;
  align-items: center;
  background: rgb(0 0 0 / 35%);
}
.app-dialog {
  display: flex;
  flex-direction: column;
  width: min(640px, 92vw);
  max-height: 80vh;
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
h2 {
  margin: var(--space-4) 0 var(--space-2);
  font-size: 12px;
  text-transform: uppercase;
  color: var(--text-muted);
}
header button {
  background: none;
  border: 0;
  font-size: 20px;
  cursor: pointer;
}
input[type='search'] {
  margin: var(--space-3) var(--space-4) 0;
  padding: var(--space-2);
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}
.body {
  padding: var(--space-3) var(--space-4) var(--space-4);
  overflow: auto;
}
.row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-2) 0;
}
select,
textarea,
.body button {
  padding: 4px var(--space-2);
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}
textarea {
  width: 100%;
  font-family: var(--font-mono);
}
.shortcuts {
  margin: 0;
  padding: 0;
  list-style: none;
}
.shortcuts li {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}
.shortcuts li span {
  flex: 1;
}
kbd {
  color: var(--text-muted);
  font-family: var(--font-ui);
}
.conflict {
  flex-basis: 100%;
  color: var(--danger);
}
.hint {
  display: block;
  color: var(--text-muted);
}
.mirror {
  width: 55%;
  padding: 4px var(--space-2);
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}
.link {
  margin-top: var(--space-3);
}
</style>
