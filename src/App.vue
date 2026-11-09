<script setup lang="ts">
import { useAppStore } from './stores/app'

const app = useAppStore()
</script>

<template>
  <main>
    <h1>{{ app.name }}</h1>
    <p>v{{ app.version }}</p>
    <RouterView />
  </main>
</template>

<style>
:root {
  color-scheme: dark;
}
body {
  margin: 0;
  font-family: system-ui, sans-serif;
  background: #14161a;
  color: #e6e8eb;
}
main {
  padding: 2rem;
}
</style>
