<script setup lang="ts">
import type { PDFDocumentProxy, PDFPageProxy } from 'pdfjs-dist'
import * as pdfjs from 'pdfjs-dist/legacy/build/pdf.mjs'
import workerUrl from 'pdfjs-dist/legacy/build/pdf.worker.min.mjs?url'
import 'pdfjs-dist/web/pdf_viewer.css'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { pdfRead } from '../compile/api'
import {
  anchorAt,
  layoutPages,
  percentOf,
  scaleFor,
  scrollForAnchor,
  stepZoom,
  visibleRange,
  ZOOM_STEPS,
  currentPage,
  type Size,
  type ZoomMode,
} from '../pdf/layout'
import { useCompileStore, type SyncTarget } from '../stores/compile'
import AppIcon from './AppIcon.vue'
import PdfThumbs from './PdfThumbs.vue'

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl

const GAP = 8
const PAD = 12
const MAX_PARALLEL = 2
const ZOOM_KEY = 'texlocal.pdf.zoom.v1'

const { t } = useI18n()
const compile = useCompileStore()

// ───────────────────────── estado ─────────────────────────

const scroller = ref<HTMLElement | null>(null)
const doc = shallowRef<PDFDocumentProxy | null>(null)
const status = ref<'empty' | 'loading' | 'ready' | 'error'>('empty')
const errorMessage = ref('')
const sizes = ref<Size[]>([])
const box = ref<Size>({ w: 600, h: 800 })
const scrollTop = ref(0)
const showThumbs = ref(false)
/** Muda a cada documento carregado (invalida miniaturas e páginas desenhadas). */
const docEpoch = ref(0)

function loadMode(): ZoomMode {
  try {
    const raw = localStorage.getItem(ZOOM_KEY)
    if (raw) {
      const m = JSON.parse(raw) as ZoomMode
      if (m.kind === 'fitWidth' || m.kind === 'fitPage') return m
      if (m.kind === 'percent' && typeof m.value === 'number') return m
    }
  } catch {
    // padrão
  }
  return { kind: 'fitWidth' }
}
const mode = ref<ZoomMode>(loadMode())
watch(mode, (m) => {
  try {
    localStorage.setItem(ZOOM_KEY, JSON.stringify(m))
  } catch {
    // ignora
  }
})

const largest = computed<Size>(() =>
  sizes.value.reduce((a, s) => ({ w: Math.max(a.w, s.w), h: Math.max(a.h, s.h) }), { w: 0, h: 0 }),
)
const scale = computed(() =>
  scaleFor(mode.value, largest.value, { w: box.value.w - 2 * PAD, h: box.value.h - 2 * PAD }),
)
const heightsAt = (s: number) => sizes.value.map((p) => p.h * s)
const heights = computed(() => heightsAt(scale.value))
const widths = computed(() => sizes.value.map((p) => p.w * scale.value))
const layout = computed(() => layoutPages(heights.value, GAP, PAD))
const percent = computed(() => percentOf(scale.value))
const current = computed(
  () => currentPage(layout.value, heights.value, scrollTop.value, box.value.h) + 1,
)
const pageInput = ref('1')
watch(current, (n) => (pageInput.value = String(n)))

// ───────────────────────── carregamento (com double buffering) ─────────────────────────

let docGen = 0

async function load() {
  const path = compile.pdfPath
  if (!path) {
    status.value = 'empty'
    return
  }
  const gen = ++docGen
  if (!doc.value) status.value = 'loading'
  let data: ArrayBuffer
  try {
    data = await pdfRead(path)
  } catch {
    // Ainda não há PDF (nunca compilado): mantém o que estiver na tela.
    if (!doc.value) status.value = 'empty'
    return
  }
  let next: PDFDocumentProxy
  try {
    next = await pdfjs.getDocument({ data: new Uint8Array(data) }).promise
  } catch (e) {
    if (gen !== docGen) return
    status.value = 'error'
    errorMessage.value = e instanceof Error ? e.message : String(e)
    return
  }
  if (gen !== docGen) {
    void next.destroy()
    return
  }
  const first = await next.getPage(1)
  const vp = first.getViewport({ scale: 1 })
  if (gen !== docGen) {
    void next.destroy()
    return
  }

  // Preserva a posição: o DOM antigo continua na tela até as páginas novas ficarem prontas.
  const keepTop = scroller.value?.scrollTop ?? 0
  const old = doc.value
  doc.value = next
  docEpoch.value++
  sizes.value = Array.from({ length: next.numPages }, () => ({ w: vp.width, h: vp.height }))
  status.value = 'ready'
  invalidateRender()
  await nextTick()
  if (scroller.value) scroller.value.scrollTop = keepTop
  void old?.destroy()
  scheduleRender()
}

// ───────────────────────── renderização lazy ─────────────────────────

/** Página -> época em que foi desenhada. */
const rendered = new Map<number, number>()
/** Páginas que têm conteúdo no DOM (mesmo desatualizado): são liberadas ao sair da janela. */
const painted = new Set<number>()
const inflight = new Map<number, { epoch: number; cancel: () => void }>()
const textLayers = new Map<number, InstanceType<typeof pdfjs.TextLayer>>()
let epoch = 0
let active = 0
let queue: number[] = []
let raf = 0

function pageEl(n: number): HTMLElement | null {
  return scroller.value?.querySelector<HTMLElement>(`.page[data-page="${n}"]`) ?? null
}

/** Descarta o que foi desenhado (novo documento ou nova escala), mantendo o DOM antigo visível. */
function invalidateRender() {
  epoch++
  for (const { cancel } of inflight.values()) cancel()
  inflight.clear()
  queue = []
  rendered.clear()
}

function scheduleRender() {
  cancelAnimationFrame(raf)
  raf = requestAnimationFrame(updateVisible)
}

function freePage(n: number) {
  inflight.get(n)?.cancel()
  inflight.delete(n)
  rendered.delete(n)
  painted.delete(n)
  textLayers.delete(n)
  pageEl(n)
    ?.querySelectorAll(':scope > canvas, :scope > .textLayer, :scope > .linkLayer')
    .forEach((x) => x.remove())
}

function updateVisible() {
  const el = scroller.value
  if (!el || !doc.value) return
  const vh = el.clientHeight
  const near = visibleRange(layout.value, heights.value, el.scrollTop, vh, vh)
  const keep = visibleRange(layout.value, heights.value, el.scrollTop, vh, vh * 4)
  // Páginas longe da janela liberam memória.
  for (const n of [...painted]) {
    if (!keep || n - 1 < keep[0] || n - 1 > keep[1]) freePage(n)
  }
  if (!near) return
  const middle = current.value
  const wanted: number[] = []
  for (let i = near[0]; i <= near[1]; i++) {
    const n = i + 1
    if (rendered.get(n) !== epoch && inflight.get(n)?.epoch !== epoch && !queue.includes(n)) {
      wanted.push(n)
    }
  }
  queue.push(...wanted)
  queue.sort((a, b) => Math.abs(a - middle) - Math.abs(b - middle))
  pump()
}

function pump() {
  while (active < MAX_PARALLEL && queue.length) {
    const n = queue.shift() as number
    void renderPage(n)
  }
}

interface LinkAnnotation {
  subtype: string
  rect?: number[]
  dest?: string | unknown[]
  url?: string
  unsafeUrl?: string
}

async function renderPage(n: number) {
  const d = doc.value
  if (!d) return
  const myEpoch = epoch
  const stale = () => myEpoch !== epoch || d !== doc.value
  const token = { epoch: myEpoch, cancel: () => {} }
  inflight.set(n, token)
  active++
  try {
    const page = await d.getPage(n)
    if (stale()) return
    const viewport = page.getViewport({ scale: scale.value })

    // Páginas com tamanho diferente da primeira reajustam o layout.
    const real = page.getViewport({ scale: 1 })
    const known = sizes.value[n - 1]
    if (known && (Math.abs(known.w - real.width) > 0.5 || Math.abs(known.h - real.height) > 0.5)) {
      sizes.value = sizes.value.map((s, i) => (i === n - 1 ? { w: real.width, h: real.height } : s))
    }

    const dpr = window.devicePixelRatio || 1
    const canvas = document.createElement('canvas')
    canvas.width = Math.floor(viewport.width * dpr)
    canvas.height = Math.floor(viewport.height * dpr)
    const ctx = canvas.getContext('2d', { alpha: false })
    if (!ctx) return
    const task = page.render({
      canvasContext: ctx,
      viewport,
      transform: dpr !== 1 ? [dpr, 0, 0, dpr, 0, 0] : undefined,
      background: '#ffffff',
    })
    token.cancel = () => task.cancel()
    await task.promise
    if (stale()) return

    const host = pageEl(n)
    if (!host) return
    // Troca atômica: a página antiga só some quando a nova está pronta (sem piscar).
    const oldCanvas = host.querySelector(':scope > canvas')
    if (oldCanvas) oldCanvas.replaceWith(canvas)
    else host.prepend(canvas)
    painted.add(n)

    const textDiv = document.createElement('div')
    textDiv.className = 'textLayer'
    textDiv.style.setProperty('--scale-factor', String(viewport.scale))
    const layer = new pdfjs.TextLayer({
      textContentSource: page.streamTextContent(),
      container: textDiv,
      viewport,
    })
    token.cancel = () => layer.cancel()
    await layer.render()
    if (stale()) return
    const links = await buildLinks(page, viewport)
    if (stale()) return

    host.querySelectorAll(':scope > .textLayer, :scope > .linkLayer').forEach((x) => x.remove())
    host.append(textDiv, links)
    textLayers.set(n, layer)
    painted.add(n)
    rendered.set(n, myEpoch)
    paintHighlights(n)
  } catch (e) {
    if (!(e instanceof Error) || e.name !== 'RenderingCancelledException') {
      console.warn('falha ao desenhar a página', n, e)
    }
  } finally {
    if (inflight.get(n) === token) inflight.delete(n)
    active--
    pump()
  }
}

interface LinkBox {
  left: number
  top: number
  right: number
  bottom: number
  key: string
  annotation: LinkAnnotation
  /** Fragmentos da mesma citação (quebrada em linhas) compartilham o grupo. */
  group?: number
}

interface TextBox {
  left: number
  right: number
  top: number
  bottom: number
}

/** Caixas (em pixels da viewport) dos trechos de texto da página. */
async function textBoxes(
  page: PDFPageProxy,
  viewport: ReturnType<PDFPageProxy['getViewport']>,
): Promise<TextBox[]> {
  const content = await page.getTextContent()
  const out: TextBox[] = []
  for (const item of content.items) {
    if (!('str' in item) || !item.str.trim()) continue
    const [, , , , e, f] = item.transform as number[]
    const [x1, y1] = viewport.convertToViewportPoint(e, f)
    const [x2] = viewport.convertToViewportPoint(e + item.width, f)
    const h = item.height * viewport.scale
    out.push({
      left: Math.min(x1, x2),
      right: Math.max(x1, x2),
      top: y1 - h,
      bottom: y1 + h * 0.25,
    })
  }
  return out
}

/**
 * O pdfTeX registra o trecho de um link que foi quebrado de linha com a largura da caixa da
 * linha, que pode sobrar para a esquerda do texto. Corta a área ao texto que ela realmente cobre.
 */
function clipToText(box: LinkBox, texts: TextBox[]): void {
  let left = Infinity
  let right = -Infinity
  const mid = (box.top + box.bottom) / 2
  for (const t of texts) {
    if (mid < t.top || mid > t.bottom) continue
    if (t.right < box.left || t.left > box.right) continue
    left = Math.min(left, t.left)
    right = Math.max(right, t.right)
  }
  if (left === Infinity) return
  box.left = Math.max(box.left, left)
  box.right = Math.min(box.right, right)
}

function assignGroups(boxes: LinkBox[]): void {
  let group = 0
  let prev: LinkBox | undefined
  for (const box of boxes) {
    const height = box.bottom - box.top
    const continues =
      prev && prev.key === box.key && box.top >= prev.top && box.top - prev.bottom <= height * 2
    if (!continues) group++
    box.group = group
    prev = box
  }
}

/**
 * Citações como "(POLYA, 2006)" saem do LaTeX como vários links colados (autor e ano), todos
 * para a mesma referência. Junta os vizinhos da mesma linha e mesmo destino em uma só área,
 * para o realce e o clique cobrirem a citação inteira.
 */
function mergeLinkBoxes(boxes: LinkBox[]): LinkBox[] {
  const merged: LinkBox[] = []
  for (const box of boxes) {
    const prev = merged[merged.length - 1]
    const height = box.bottom - box.top
    if (
      prev &&
      prev.key === box.key &&
      // mesma linha: as faixas verticais se sobrepõem quase por completo
      Math.min(prev.bottom, box.bottom) - Math.max(prev.top, box.top) > height * 0.6 &&
      // vizinhos: o vão (", ") cabe em ~1,5 altura de linha
      box.left - prev.right >= -2 &&
      box.left - prev.right <= height * 1.5
    ) {
      prev.right = Math.max(prev.right, box.right)
      prev.top = Math.min(prev.top, box.top)
      prev.bottom = Math.max(prev.bottom, box.bottom)
    } else {
      merged.push({ ...box })
    }
  }
  return merged
}

async function buildLinks(page: PDFPageProxy, viewport: ReturnType<PDFPageProxy['getViewport']>) {
  const layer = document.createElement('div')
  layer.className = 'linkLayer'
  const annotations = (await page.getAnnotations({ intent: 'display' })) as LinkAnnotation[]
  const boxes: LinkBox[] = []
  for (const a of annotations) {
    if (a.subtype !== 'Link' || !a.rect || !(a.dest || a.url || a.unsafeUrl)) continue
    const [x1, y1, x2, y2] = viewport.convertToViewportRectangle(a.rect as number[])
    boxes.push({
      left: Math.min(x1, x2),
      top: Math.min(y1, y2),
      right: Math.max(x1, x2),
      bottom: Math.max(y1, y2),
      key: JSON.stringify(a.dest ?? a.url ?? a.unsafeUrl),
      annotation: a,
    })
  }
  const texts = await textBoxes(page, viewport)
  const merged = mergeLinkBoxes(boxes)
  for (const box of merged) clipToText(box, texts)
  assignGroups(merged)
  for (const box of merged) {
    const a = box.annotation
    const link = document.createElement('a')
    link.href = '#'
    link.dataset.group = String(box.group)
    // Citação quebrada em duas linhas: o realce acompanha todos os fragmentos.
    const setHover = (on: boolean) =>
      layer
        .querySelectorAll(`a[data-group="${box.group}"]`)
        .forEach((el) => el.classList.toggle('hover', on))
    link.addEventListener('mouseenter', () => setHover(true))
    link.addEventListener('mouseleave', () => setHover(false))
    link.style.left = `${box.left}px`
    link.style.top = `${box.top}px`
    link.style.width = `${box.right - box.left}px`
    link.style.height = `${box.bottom - box.top}px`
    link.addEventListener('click', (ev) => {
      // Ctrl/Cmd+clique é a busca inversa do SyncTeX, não um link.
      if (ev.ctrlKey || ev.metaKey) return
      ev.preventDefault()
      ev.stopPropagation()
      if (a.dest) void goToDestination(a.dest)
      else window.open(a.url ?? a.unsafeUrl, '_blank', 'noopener')
    })
    layer.append(link)
  }
  return layer
}

async function goToDestination(dest: string | unknown[]) {
  const d = doc.value
  if (!d) return
  try {
    const explicit = typeof dest === 'string' ? await d.getDestination(dest) : dest
    if (!explicit) return
    const ref = explicit[0]
    const index =
      typeof ref === 'object' && ref !== null ? await d.getPageIndex(ref as never) : Number(ref)
    // [ref /XYZ left top zoom]: leva o ponto indicado ao topo da janela.
    let offset = 0
    if ((explicit[1] as { name?: string })?.name === 'XYZ' && typeof explicit[3] === 'number') {
      const page = await d.getPage(index + 1)
      const vp = page.getViewport({ scale: scale.value })
      offset = vp.convertToViewportPoint(Number(explicit[2] ?? 0), explicit[3])[1]
    }
    scrollToPage(index + 1, offset)
  } catch {
    // destino inválido: ignora
  }
}

// ───────────────────────── rolagem e zoom ─────────────────────────

function onScroll() {
  const el = scroller.value
  if (!el) return
  scrollTop.value = el.scrollTop
  scheduleRender()
}

function scrollToPage(n: number, offset = 0) {
  const el = scroller.value
  if (!el) return
  const i = Math.min(Math.max(n, 1), sizes.value.length) - 1
  if (i < 0) return
  el.scrollTo({ top: Math.max(layout.value.tops[i] + offset - PAD, 0) })
}

function setZoom(next: ZoomMode) {
  mode.value = next
}
function zoomBy(direction: 1 | -1) {
  setZoom({ kind: 'percent', value: stepZoom(percent.value, direction) })
}
const zoomSelect = computed({
  get: () => (mode.value.kind === 'percent' ? String(mode.value.value) : mode.value.kind),
  set: (v: string) => {
    if (v === 'fitWidth' || v === 'fitPage') setZoom({ kind: v })
    else setZoom({ kind: 'percent', value: Number(v) })
  },
})

let renderTimer: ReturnType<typeof setTimeout> | undefined
watch(scale, (now, before) => {
  const el = scroller.value
  if (!el || !sizes.value.length) return
  // Mantém a mesma posição relativa no documento ao mudar o zoom.
  const anchor = anchorAt(layoutPages(heightsAt(before), GAP, PAD), heightsAt(before), el.scrollTop)
  invalidateRender()
  void nextTick(() => {
    el.scrollTop = scrollForAnchor(layout.value, heights.value, anchor)
    scrollTop.value = el.scrollTop
  })
  clearTimeout(renderTimer)
  renderTimer = setTimeout(scheduleRender, now === before ? 0 : 80)
})

watch(layout, scheduleRender)

function onWheel(e: WheelEvent) {
  if (!(e.ctrlKey || e.metaKey)) return
  e.preventDefault()
  zoomBy(e.deltaY < 0 ? 1 : -1)
}

function jumpToInput() {
  const n = Number.parseInt(pageInput.value, 10)
  if (Number.isFinite(n)) scrollToPage(n)
  else pageInput.value = String(current.value)
}

// ───────────────────────── busca de texto ─────────────────────────

const query = ref('')
interface Match {
  page: number
  /** Posição do trecho entre os itens de texto da página. */
  item: number
}
const matches = ref<Match[]>([])
const matchIndex = ref(-1)
let searchGen = 0

async function runSearch() {
  const q = query.value.trim().toLowerCase()
  const gen = ++searchGen
  matches.value = []
  matchIndex.value = -1
  repaintAll()
  const d = doc.value
  if (!q || !d) return
  const found: Match[] = []
  for (let n = 1; n <= d.numPages; n++) {
    const text = await (await d.getPage(n)).getTextContent()
    if (gen !== searchGen || d !== doc.value) return
    let item = 0
    for (const it of text.items) {
      if (!('str' in it)) continue
      if (it.str.toLowerCase().includes(q)) found.push({ page: n, item })
      item++
    }
    if (found.length && matches.value.length === 0) {
      matches.value = [...found]
      matchIndex.value = 0
      void goToMatch()
    }
  }
  matches.value = found
  if (found.length && matchIndex.value < 0) {
    matchIndex.value = 0
    void goToMatch()
  }
  repaintAll()
}

function paintHighlights(n: number) {
  const layer = textLayers.get(n)
  if (!layer) return
  const divs = layer.textDivs
  divs.forEach((d) => d.classList.remove('hit', 'current'))
  matches.value.forEach((m, i) => {
    if (m.page !== n) return
    divs[m.item]?.classList.add('hit')
    if (i === matchIndex.value) divs[m.item]?.classList.add('current')
  })
}
function repaintAll() {
  for (const n of textLayers.keys()) paintHighlights(n)
}

async function goToMatch() {
  const m = matches.value[matchIndex.value]
  if (!m) return
  scrollToPage(m.page)
  repaintAll()
  // Espera a página ficar pronta para centralizar o trecho.
  for (let tries = 0; tries < 60; tries++) {
    const span = textLayers.get(m.page)?.textDivs[m.item]
    if (span && rendered.get(m.page) === epoch) {
      span.scrollIntoView({ block: 'center', inline: 'nearest' })
      return
    }
    await new Promise((r) => setTimeout(r, 50))
  }
}

let searchTimer: ReturnType<typeof setTimeout> | undefined
function onSearchInput() {
  clearTimeout(searchTimer)
  searchTimer = setTimeout(() => void runSearch(), 250)
}

function step(delta: number) {
  if (!matches.value.length) return
  matchIndex.value = (matchIndex.value + delta + matches.value.length) % matches.value.length
  void goToMatch()
}

// ───────────────────────── SyncTeX ─────────────────────────

function onClick(e: MouseEvent) {
  if (!(e.ctrlKey || e.metaKey) || e.button !== 0) return
  inverse(e)
}

function inverse(e: MouseEvent) {
  const host = (e.target as HTMLElement).closest<HTMLElement>('.page')
  if (!host) return
  e.preventDefault()
  const rect = host.getBoundingClientRect()
  const n = Number(host.dataset.page)
  // Pixels CSS -> pontos PDF, com origem no canto superior esquerdo.
  void compile.syncToSource(
    n,
    (e.clientX - rect.left) / scale.value,
    (e.clientY - rect.top) / scale.value,
  )
}

let syncTimer: ReturnType<typeof setTimeout> | undefined
function showSyncTarget(target: SyncTarget) {
  const el = scroller.value
  const i = target.page - 1
  if (!el || i < 0 || i >= sizes.value.length) return
  const first = target.rects[0]
  const top = layout.value.tops[i] + first.y * scale.value
  el.scrollTo({ top: Math.max(top - el.clientHeight / 3, 0), behavior: 'smooth' })
  document.querySelectorAll('.sync-hit').forEach((x) => x.remove())
  const host = pageEl(target.page)
  if (!host) return
  for (const r of target.rects) {
    const mark = document.createElement('div')
    mark.className = 'sync-hit'
    mark.style.left = `${r.x * scale.value}px`
    mark.style.top = `${r.y * scale.value}px`
    mark.style.width = `${r.w * scale.value}px`
    mark.style.height = `${r.h * scale.value}px`
    host.append(mark)
  }
  clearTimeout(syncTimer)
  syncTimer = setTimeout(
    () => document.querySelectorAll('.sync-hit').forEach((x) => x.remove()),
    2500,
  )
}
watch(
  () => compile.syncTarget,
  (target) => target && showSyncTarget(target),
)

// ───────────────────────── ciclo de vida ─────────────────────────

let resizeObserver: ResizeObserver | null = null
let resizeTimer: ReturnType<typeof setTimeout> | undefined

onMounted(() => {
  const el = scroller.value
  if (el) {
    box.value = { w: el.clientWidth, h: el.clientHeight }
    if (typeof ResizeObserver !== 'undefined') {
      // Debounce: no zoom "ajustar largura", cada largura nova recalcula a escala e re-renderiza.
      resizeObserver = new ResizeObserver(() => {
        clearTimeout(resizeTimer)
        resizeTimer = setTimeout(() => {
          box.value = { w: el.clientWidth, h: el.clientHeight }
          scheduleRender()
        }, 90)
      })
      resizeObserver.observe(el)
    }
  }
  void load()
})

watch(
  () => [compile.pdfPath, compile.pdfVersion],
  () => void load(),
)

onBeforeUnmount(() => {
  cancelAnimationFrame(raf)
  clearTimeout(renderTimer)
  clearTimeout(resizeTimer)
  clearTimeout(syncTimer)
  clearTimeout(searchTimer)
  resizeObserver?.disconnect()
  invalidateRender()
  searchGen++
  void doc.value?.destroy()
})

const running = computed(() => compile.state === 'running')
const stateLabel = computed(() => t(`compile.state.${compile.state}`))
</script>

<template>
  <div class="viewer">
    <div class="toolbar" role="toolbar" :aria-label="t('pdf.toolbar')">
      <button
        v-if="!running"
        class="primary"
        :title="t('cmd.compile')"
        :aria-label="t('cmd.compile')"
        @click="compile.compile({ manual: true })"
      >
        <AppIcon name="play" :size="14" /> {{ t('compile.run') }}
      </button>
      <button
        v-else
        class="danger"
        :title="t('cmd.cancelCompile')"
        :aria-label="t('cmd.cancelCompile')"
        @click="compile.cancel()"
      >
        <AppIcon name="stop" :size="14" /> {{ t('compile.cancel') }}
      </button>
      <span class="state" :class="compile.state" aria-live="polite">{{ stateLabel }}</span>
      <span class="spacer" />

      <button
        :aria-pressed="showThumbs"
        :title="t('pdf.thumbnails')"
        :aria-label="t('pdf.thumbnails')"
        :disabled="status !== 'ready'"
        @click="showThumbs = !showThumbs"
      >
        <AppIcon name="view-grid-outline" :size="15" />
      </button>
      <label class="page-box">
        <input
          v-model="pageInput"
          inputmode="numeric"
          :aria-label="t('pdf.goToPage')"
          :disabled="status !== 'ready'"
          @keydown.enter.prevent="jumpToInput"
          @blur="jumpToInput"
        />
        <span>/ {{ sizes.length }}</span>
      </label>
      <button :title="t('pdf.zoomOut')" :aria-label="t('pdf.zoomOut')" @click="zoomBy(-1)">
        <AppIcon name="minus" :size="15" />
      </button>
      <select v-model="zoomSelect" :aria-label="t('pdf.zoom')">
        <option value="fitWidth">{{ t('pdf.fitWidth') }}</option>
        <option value="fitPage">{{ t('pdf.fitPage') }}</option>
        <option v-for="z in ZOOM_STEPS" :key="z" :value="String(z)">{{ z }}%</option>
        <option
          v-if="mode.kind === 'percent' && !ZOOM_STEPS.includes(mode.value)"
          :value="String(mode.value)"
        >
          {{ mode.value }}%
        </option>
      </select>
      <button :title="t('pdf.zoomIn')" :aria-label="t('pdf.zoomIn')" @click="zoomBy(1)">
        <AppIcon name="plus" :size="15" />
      </button>
      <span class="percent" aria-hidden="true">{{ percent }}%</span>

      <input
        v-model="query"
        class="search"
        type="search"
        :placeholder="t('pdf.search')"
        :aria-label="t('pdf.search')"
        :disabled="status !== 'ready'"
        @keydown.enter.exact.prevent="matches.length && query.trim() ? step(1) : runSearch()"
        @keydown.enter.shift.prevent="step(-1)"
        @input="onSearchInput"
      />
      <span v-if="query.trim()" class="count" aria-live="polite">
        {{ matches.length ? `${matchIndex + 1} / ${matches.length}` : t('pdf.noMatches') }}
      </span>
    </div>

    <div class="body">
      <PdfThumbs
        v-if="showThumbs && status === 'ready'"
        :doc="doc"
        :pages="sizes.length"
        :current="current"
        :epoch="docEpoch"
        @go="scrollToPage"
      />
      <div
        ref="scroller"
        class="scroller"
        tabindex="0"
        :aria-label="t('pdf.title')"
        @scroll.passive="onScroll"
        @wheel="onWheel"
        @click="onClick"
        @dblclick="inverse"
      >
        <div v-if="status !== 'ready'" class="message">
          <p v-if="status === 'loading'">{{ t('pdf.loading') }}</p>
          <p v-else-if="status === 'error'">{{ t('pdf.error') }}<br />{{ errorMessage }}</p>
          <p v-else>{{ t('pdf.placeholder') }}</p>
        </div>
        <div
          v-show="status === 'ready'"
          class="pages"
          :style="{ height: layout.total + 'px', width: Math.max(...widths, 0) + 2 * PAD + 'px' }"
        >
          <div
            v-for="(_, i) in sizes"
            :key="i"
            class="page"
            :data-page="i + 1"
            :style="{
              top: layout.tops[i] + 'px',
              width: widths[i] + 'px',
              height: heights[i] + 'px',
              left: `max(${PAD}px, calc(50% - ${widths[i] / 2}px))`,
            }"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.viewer {
  display: flex;
  flex-direction: column;
  flex: 1;
  width: 100%;
  height: 100%;
  min-height: 0;
  min-width: 0;
}
.toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-2);
  border-bottom: 1px solid var(--border);
  background: var(--bg-panel);
  font-size: 12px;
}
.toolbar button,
.toolbar select,
.toolbar input {
  background: var(--bg-elevated);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 1px var(--space-2);
  font: inherit;
}
.toolbar button {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
}
.toolbar button[aria-pressed='true'] {
  border-color: var(--accent);
}
.toolbar button:disabled,
.toolbar input:disabled {
  opacity: 0.5;
  cursor: default;
}
.toolbar .primary {
  background: var(--accent);
  color: var(--accent-contrast);
  border-color: var(--accent);
}
.toolbar .danger {
  border-color: var(--danger);
  color: var(--danger);
}
.spacer {
  flex: 1;
}
.state {
  color: var(--text-muted);
}
.state.failed,
.state.error,
.state.timedOut {
  color: var(--danger);
}
.page-box {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  color: var(--text-muted);
}
.page-box input {
  width: 3.2em;
  text-align: right;
}
.search {
  width: 130px;
}
.percent,
.count {
  color: var(--text-muted);
}
.body {
  display: flex;
  flex: 1;
  min-height: 0;
}
.scroller {
  position: relative;
  flex: 1;
  min-width: 0;
  overflow: auto;
  background: var(--bg);
}
.message {
  padding: var(--space-4);
  color: var(--text-muted);
}
.pages {
  position: relative;
  min-width: 100%;
}
.page {
  position: absolute;
  background: #fff;
  box-shadow: 0 1px 4px rgb(0 0 0 / 40%);
}
.page :deep(canvas) {
  position: absolute;
  inset: 0;
  display: block;
  width: 100%;
  height: 100%;
}
.page :deep(.textLayer) {
  position: absolute;
  inset: 0;
}
.page :deep(.textLayer .hit) {
  background: rgb(255 214 0 / 45%);
  border-radius: 2px;
}
.page :deep(.textLayer .hit.current) {
  background: rgb(255 140 0 / 65%);
}
.page :deep(.linkLayer a) {
  position: absolute;
  display: block;
}
.page :deep(.linkLayer a) {
  border-radius: 3px;
  transition:
    background-color 0.12s ease,
    box-shadow 0.12s ease;
}
.page :deep(.linkLayer a:hover),
.page :deep(.linkLayer a.hover) {
  background: rgb(106 166 255 / 22%);
  box-shadow: 0 0 0 1px rgb(106 166 255 / 55%);
}
.page :deep(.sync-hit) {
  position: absolute;
  pointer-events: none;
  background: rgb(255 214 0 / 40%);
  outline: 2px solid rgb(255 160 0 / 80%);
  animation: sync-fade 2.5s ease-out forwards;
}
@keyframes sync-fade {
  0%,
  60% {
    opacity: 1;
  }
  100% {
    opacity: 0;
  }
}
</style>
