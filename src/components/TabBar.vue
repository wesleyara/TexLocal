<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useEditorStore } from '../stores/editor'

const { t } = useI18n()
const editor = useEditorStore()

const dragFrom = ref<number | null>(null)
const dragOver = ref<number | null>(null)
const menu = ref<{ path: string; x: number; y: number } | null>(null)

function onDrop(to: number) {
  if (dragFrom.value !== null) editor.move(dragFrom.value, to)
  dragFrom.value = null
  dragOver.value = null
}

function openMenu(e: MouseEvent, path: string) {
  menu.value = { path, x: e.clientX, y: e.clientY }
}

async function run(action: 'close' | 'others' | 'right') {
  const m = menu.value
  menu.value = null
  if (!m) return
  if (action === 'close') await editor.close(m.path)
  else if (action === 'others') await editor.closeOthers(m.path)
  else await editor.closeToRight(m.path)
}

function onKey(e: KeyboardEvent, i: number) {
  const last = editor.tabs.length - 1
  if (e.key === 'ArrowRight' && i < last) editor.activate(editor.tabs[i + 1].path)
  else if (e.key === 'ArrowLeft' && i > 0) editor.activate(editor.tabs[i - 1].path)
  else if (e.key === 'Enter' || e.key === ' ') editor.activate(editor.tabs[i].path)
  else return
  e.preventDefault()
}
</script>

<template>
  <div v-if="editor.tabs.length" class="tabs" role="tablist" :aria-label="t('editor.tabs')">
    <div
      v-for="(tab, i) in editor.tabs"
      :key="tab.path"
      class="tab"
      :class="{ active: tab.path === editor.active, over: dragOver === i }"
      role="tab"
      :aria-selected="tab.path === editor.active"
      :tabindex="tab.path === editor.active ? 0 : -1"
      :title="tab.path"
      draggable="true"
      @click="editor.activate(tab.path)"
      @auxclick.middle.prevent="editor.close(tab.path)"
      @contextmenu.prevent="openMenu($event, tab.path)"
      @keydown="onKey($event, i)"
      @dragstart="dragFrom = i"
      @dragover.prevent="dragOver = i"
      @dragleave="dragOver = null"
      @drop.prevent="onDrop(i)"
      @dragend="((dragFrom = null), (dragOver = null))"
    >
      <span class="name">{{ tab.name }}</span>
      <span
        v-if="tab.dirty"
        class="dot"
        :title="t('editor.unsaved')"
        role="img"
        :aria-label="t('editor.unsaved')"
        >●</span
      >
      <button
        class="close"
        :aria-label="t('editor.closeTab', { name: tab.name })"
        tabindex="-1"
        @click.stop="editor.close(tab.path)"
      >
        ×
      </button>
    </div>
  </div>
  <template v-if="menu">
    <div class="backdrop" @mousedown="menu = null" @contextmenu.prevent="menu = null" />
    <ul class="menu" role="menu" :style="{ left: menu.x + 'px', top: menu.y + 'px' }">
      <li role="menuitem" @click="run('close')">{{ t('editor.tab.close') }}</li>
      <li role="menuitem" @click="run('others')">{{ t('editor.tab.closeOthers') }}</li>
      <li role="menuitem" @click="run('right')">{{ t('editor.tab.closeRight') }}</li>
    </ul>
  </template>
</template>

<style scoped>
.tabs {
  display: flex;
  flex: none;
  overflow-x: auto;
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border);
}
.tab {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: 0 var(--space-2) 0 var(--space-3);
  height: 32px;
  flex: none;
  color: var(--text-muted);
  border-right: 1px solid var(--border);
  cursor: pointer;
  user-select: none;
}
.tab.active {
  color: var(--text);
  background: var(--bg);
  box-shadow: inset 0 -2px 0 var(--accent);
}
.tab.over {
  outline: 1px dashed var(--accent);
  outline-offset: -2px;
}
.dot {
  color: var(--accent);
  font-size: 10px;
}
.close {
  background: none;
  border: 0;
  color: inherit;
  font-size: 16px;
  line-height: 1;
  border-radius: var(--radius-sm);
  cursor: pointer;
  width: 18px;
  height: 18px;
}
.close:hover {
  background: var(--bg-hover);
}
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 40;
}
.menu {
  position: fixed;
  z-index: 41;
  margin: 0;
  padding: var(--space-1);
  min-width: 180px;
  list-style: none;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
}
.menu li {
  padding: 5px var(--space-3);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.menu li:hover {
  background: var(--bg-hover);
}
</style>
