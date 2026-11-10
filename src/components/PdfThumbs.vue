<script setup lang="ts">
import type { PDFDocumentProxy } from 'pdfjs-dist'
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  doc: PDFDocumentProxy | null
  pages: number
  /** Página atual (1-based). */
  current: number
  /** Muda a cada documento novo, para redesenhar as miniaturas. */
  epoch: number
}>()
const emit = defineEmits<{ go: [page: number] }>()

const { t } = useI18n()
const list = ref<HTMLElement | null>(null)
const THUMB_WIDTH = 96

let observer: IntersectionObserver | null = null
const drawn = new Map<number, number>()

async function draw(n: number, canvas: HTMLCanvasElement) {
  const doc = props.doc
  if (!doc || drawn.get(n) === props.epoch) return
  drawn.set(n, props.epoch)
  try {
    const page = await doc.getPage(n)
    if (doc !== props.doc) return
    const base = page.getViewport({ scale: 1 })
    const viewport = page.getViewport({ scale: THUMB_WIDTH / base.width })
    const dpr = window.devicePixelRatio || 1
    canvas.width = Math.floor(viewport.width * dpr)
    canvas.height = Math.floor(viewport.height * dpr)
    const ctx = canvas.getContext('2d', { alpha: false })
    if (!ctx) return
    await page.render({
      canvasContext: ctx,
      viewport,
      transform: dpr !== 1 ? [dpr, 0, 0, dpr, 0, 0] : undefined,
      background: '#ffffff',
    }).promise
  } catch {
    drawn.delete(n)
  }
}

function observe() {
  observer?.disconnect()
  drawn.clear()
  const root = list.value
  if (!root || typeof IntersectionObserver === 'undefined') return
  observer = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue
        const canvas = e.target.querySelector('canvas')
        const n = Number((e.target as HTMLElement).dataset.page)
        if (canvas && n) void draw(n, canvas)
      }
    },
    { root, rootMargin: '300px' },
  )
  root.querySelectorAll('[data-page]').forEach((el) => observer?.observe(el))
}

watch(
  () => [props.epoch, props.pages],
  () => void nextTick(observe),
  { immediate: true, flush: 'post' },
)

watch(
  () => props.current,
  (n) => list.value?.querySelector(`[data-page="${n}"]`)?.scrollIntoView({ block: 'nearest' }),
)

onBeforeUnmount(() => observer?.disconnect())
</script>

<template>
  <div ref="list" class="thumbs" role="listbox" :aria-label="t('pdf.thumbnails')">
    <button
      v-for="n in pages"
      :key="`${epoch}-${n}`"
      class="thumb"
      :class="{ active: n === current }"
      role="option"
      :aria-selected="n === current"
      :data-page="n"
      :aria-label="t('pdf.page', { n })"
      @click="emit('go', n)"
    >
      <canvas :style="{ width: THUMB_WIDTH + 'px' }" />
      <span>{{ n }}</span>
    </button>
  </div>
</template>

<style scoped>
.thumbs {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-2);
  width: 120px;
  flex: none;
  padding: var(--space-2);
  overflow-y: auto;
  background: var(--bg);
  border-right: 1px solid var(--border);
}
.thumb {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  min-height: 120px;
  padding: var(--space-1);
  background: none;
  border: 2px solid transparent;
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  cursor: pointer;
}
.thumb.active {
  border-color: var(--accent);
  color: var(--text);
}
canvas {
  display: block;
  background: #fff;
}
</style>
