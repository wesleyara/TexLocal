<script setup lang="ts">
import { onMounted, watch, watchEffect } from 'vue'
import AppShell from './components/AppShell.vue'
import { setLocale } from './i18n'
import { useEditorStore } from './stores/editor'
import { useProjectStore } from './stores/project'
import { useSettingsStore } from './stores/settings'
import { applyZoom } from './settings/zoom'
import { useNotificationsStore } from './stores/notifications'
import { useUiStore } from './stores/ui'

const settings = useSettingsStore()
const ui = useUiStore()
const project = useProjectStore()
const editor = useEditorStore()
const notifications = useNotificationsStore()

onMounted(async () => {
  await settings.load()
  // Primeira execução: o assistente abre até o usuário escolher ou pular.
  if (settings.effective.texSource === 'unset') ui.wizardOpen = true
  // Reabre o último projeto e as abas que estavam abertas.
  const last = project.lastRoot()
  if (last) {
    try {
      await project.open(last)
      await editor.restoreSession()
    } catch {
      // pasta removida ou fora do app desktop: segue sem projeto
    }
  }
})

const dark = window.matchMedia('(prefers-color-scheme: dark)')

watchEffect(() => {
  const theme = settings.effective.theme
  document.documentElement.dataset.theme =
    theme === 'system' ? (dark.matches ? 'dark' : 'light') : theme
})

watchEffect(() => setLocale(settings.effective.locale))

watchEffect(() => void applyZoom(settings.effective.zoomLevel))

// Ao salvar o settings.json aberto no editor, as alterações valem na hora.
watch(
  () => editor.saved,
  async (s) => {
    const file = await settings.filePath()
    if (!file || s.path.replace(/\\/g, '/') !== file.replace(/\\/g, '/')) return
    const error = await settings.reload()
    if (error) notifications.notify('error', error)
  },
)
</script>

<template>
  <AppShell />
</template>
