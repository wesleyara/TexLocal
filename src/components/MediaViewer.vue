<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { fileReadBytes } from '../project/api'
import type { MediaKind } from '../stores/editor'

const props = defineProps<{ path: string; kind: MediaKind; version?: number }>()
const { t } = useI18n()

const url = ref<string | null>(null)
const error = ref('')
const fit = ref(true)
const size = ref<{ w: number; h: number } | null>(null)

const MIME: Record<string, string> = {
  png: 'image/png',
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  gif: 'image/gif',
  webp: 'image/webp',
  bmp: 'image/bmp',
  svg: 'image/svg+xml',
  pdf: 'application/pdf',
}

function release() {
  if (url.value) URL.revokeObjectURL(url.value)
  url.value = null
}

async function load() {
  release()
  error.value = ''
  size.value = null
  const path = props.path
  try {
    const bytes = await fileReadBytes(path)
    if (path !== props.path) return
    const ext = path.slice(path.lastIndexOf('.') + 1).toLowerCase()
    url.value = URL.createObjectURL(new Blob([bytes], { type: MIME[ext] ?? '' }))
  } catch (e) {
    error.value = String(e)
  }
}

watch(() => [props.path, props.version], load, { immediate: true })
onBeforeUnmount(release)

function onLoad(e: Event) {
  const img = e.target as HTMLImageElement
  size.value = { w: img.naturalWidth, h: img.naturalHeight }
}
</script>

<template>
  <div class="media">
    <p v-if="error" class="error" role="alert">{{ error }}</p>
    <template v-else-if="url">
      <iframe v-if="kind === 'pdf'" :src="url" :title="path" class="pdf" />
      <template v-else>
        <div class="bar">
          <span v-if="size" class="muted">{{ size.w }} × {{ size.h }} px</span>
          <button @click="fit = !fit">
            {{ fit ? t('media.actualSize') : t('media.fit') }}
          </button>
        </div>
        <div class="canvas" :class="{ fit }">
          <img :src="url" :alt="path" @load="onLoad" />
        </div>
      </template>
    </template>
  </div>
</template>

<style scoped>
.media {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  background: var(--bg-panel);
}
.pdf {
  flex: 1;
  width: 100%;
  border: 0;
  background: #fff;
}
.bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  border-bottom: 1px solid var(--border);
  font-size: 12px;
}
.muted {
  color: var(--text-muted);
}
button {
  padding: 2px var(--space-3);
  background: var(--bg-hover);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.canvas {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: flex;
  padding: var(--space-3);
  /* tabuleiro: deixa ver transparência */
  background-image:
    linear-gradient(45deg, rgb(128 128 128 / 12%) 25%, transparent 25%),
    linear-gradient(-45deg, rgb(128 128 128 / 12%) 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, rgb(128 128 128 / 12%) 75%),
    linear-gradient(-45deg, transparent 75%, rgb(128 128 128 / 12%) 75%);
  background-size: 16px 16px;
  background-position:
    0 0,
    0 8px,
    8px -8px,
    -8px 0;
}
.canvas img {
  margin: auto;
}
.canvas.fit img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}
.error {
  margin: var(--space-4);
  color: var(--danger);
}
</style>
