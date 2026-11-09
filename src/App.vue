<script setup lang="ts">
import { onMounted, watchEffect } from 'vue'
import AppShell from './components/AppShell.vue'
import { setLocale } from './i18n'
import { useSettingsStore } from './stores/settings'
import { useUiStore } from './stores/ui'

const settings = useSettingsStore()
const ui = useUiStore()

onMounted(async () => {
  await settings.load()
  // Primeira execução: o assistente abre até o usuário escolher ou pular.
  if (settings.effective.texSource === 'unset') ui.wizardOpen = true
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
