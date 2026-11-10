<script setup lang="ts">
import { onMounted, watchEffect } from 'vue'
import AppShell from './components/AppShell.vue'
import { setLocale } from './i18n'
import { useEditorStore } from './stores/editor'
import { useProjectStore } from './stores/project'
import { useSettingsStore } from './stores/settings'
import { useUiStore } from './stores/ui'

const settings = useSettingsStore()
const ui = useUiStore()
const project = useProjectStore()
const editor = useEditorStore()

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
</script>

<template>
  <AppShell />
</template>
