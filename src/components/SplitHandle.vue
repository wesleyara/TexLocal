<script setup lang="ts">
const props = defineProps<{
  orientation: 'vertical' | 'horizontal'
  label: string
}>()

const emit = defineEmits<{ resize: [delta: number] }>()

function onPointerDown(e: PointerEvent) {
  const target = e.currentTarget as HTMLElement
  target.setPointerCapture(e.pointerId)
  let last = props.orientation === 'vertical' ? e.clientX : e.clientY
  const move = (ev: PointerEvent) => {
    const pos = props.orientation === 'vertical' ? ev.clientX : ev.clientY
    emit('resize', pos - last)
    last = pos
  }
  const up = () => {
    target.removeEventListener('pointermove', move)
    target.removeEventListener('pointerup', up)
  }
  target.addEventListener('pointermove', move)
  target.addEventListener('pointerup', up)
}

function onKey(e: KeyboardEvent) {
  const step = e.shiftKey ? 40 : 10
  const keys: Record<string, number> =
    props.orientation === 'vertical'
      ? { ArrowLeft: -step, ArrowRight: step }
      : { ArrowUp: -step, ArrowDown: step }
  const delta = keys[e.key]
  if (delta !== undefined) {
    e.preventDefault()
    emit('resize', delta)
  }
}
</script>

<template>
  <div
    class="handle"
    :class="orientation"
    role="separator"
    :aria-orientation="orientation"
    :aria-label="label"
    tabindex="0"
    @pointerdown="onPointerDown"
    @keydown="onKey"
  />
</template>

<style scoped>
.handle {
  flex: none;
  background: var(--border);
  touch-action: none;
}
.handle.vertical {
  width: 4px;
  cursor: col-resize;
}
.handle.horizontal {
  height: 4px;
  cursor: row-resize;
}
.handle:hover,
.handle:focus-visible {
  background: var(--accent);
}
</style>
