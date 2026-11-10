<script setup lang="ts">
import { computed } from 'vue'
import { ICONS } from '../icons/generated'

const props = withDefaults(
  defineProps<{
    /** Nome curto de interface (`play`) ou chave completa (`mit:tex`). */
    name: string
    size?: number
  }>(),
  { size: 16 },
)

const icon = computed(() => ICONS[props.name.includes(':') ? props.name : `mdi:${props.name}`])
// Ícones de interface herdam a cor do texto; os de arquivo trazem as próprias cores.
const mono = computed(() => !props.name.startsWith('mit:'))
</script>

<template>
  <svg
    v-if="icon"
    class="app-icon"
    :width="size"
    :height="size"
    :viewBox="`0 0 ${icon.w} ${icon.h}`"
    :fill="mono ? 'currentColor' : undefined"
    aria-hidden="true"
    focusable="false"
    v-html="icon.body"
  />
</template>

<style scoped>
.app-icon {
  flex: none;
  vertical-align: -0.2em;
}
</style>
