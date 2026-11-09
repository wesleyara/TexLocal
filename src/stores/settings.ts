import { invoke } from '@tauri-apps/api/core'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
  DEFAULT_SETTINGS,
  resolveSettings,
  validateSettings,
  type Settings,
} from '../settings/schema'

const LS_KEY = 'texlocal.settings.v1'

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

async function readGlobal(): Promise<Settings> {
  if (isTauri()) return validateSettings(await invoke('get_settings'))
  try {
    const raw = localStorage.getItem(LS_KEY)
    if (raw) return validateSettings(JSON.parse(raw))
  } catch {
    // corrompido: volta ao padrão
  }
  return { ...DEFAULT_SETTINGS }
}

async function writeGlobal(settings: Settings): Promise<void> {
  if (isTauri()) {
    await invoke('save_settings', { settings })
    return
  }
  localStorage.setItem(LS_KEY, JSON.stringify(settings))
}

export const useSettingsStore = defineStore('settings', () => {
  const global = ref<Settings>({ ...DEFAULT_SETTINGS })
  /** Overrides do projeto (`[editor]` do texlocal.toml) — preenchido a partir do EP08. */
  const project = ref<Partial<Settings>>({})
  const loaded = ref(false)

  const effective = computed(() => resolveSettings(DEFAULT_SETTINGS, global.value, project.value))

  async function load() {
    global.value = await readGlobal()
    loaded.value = true
  }

  /** Atualiza otimisticamente; retorna false (e reverte) se a gravação falhar. */
  async function update(patch: Partial<Settings>): Promise<boolean> {
    const previous = global.value
    const next = validateSettings({ ...previous, ...patch })
    global.value = next
    try {
      await writeGlobal(next)
      return true
    } catch {
      global.value = previous
      return false
    }
  }

  return { global, project, effective, loaded, load, update }
})
