<script setup lang="ts">
import { onMounted, watchEffect } from 'vue'
import AppShell from './components/AppShell.vue'
import { setLocale } from './i18n'
import { useSettingsStore } from './stores/settings'

const settings = useSettingsStore()

onMounted(() => void settings.load())

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
