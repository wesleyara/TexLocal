<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { currentOutlineIndex } from '../project/model'
import { useEditorStore } from '../stores/editor'
import { useProjectStore } from '../stores/project'
import AppIcon from './AppIcon.vue'

const { t } = useI18n()
const project = useProjectStore()
const editor = useEditorStore()

const current = computed(() =>
  editor.active ? currentOutlineIndex(project.outline, editor.active, editor.cursor.line - 1) : -1,
)

const ICON = { section: 'format-section', figure: 'image-outline', table: 'table' } as const

function go(i: number) {
  const it = project.outline[i]
  void editor.openFile(it.file, { line: it.line, col: 0 })
}
</script>

<template>
  <section class="outline" :aria-label="t('outline.title')">
    <h2>{{ t('outline.title') }}</h2>
    <p v-if="!project.outline.length" class="muted">{{ t('outline.empty') }}</p>
    <ul v-else>
      <li
        v-for="(it, i) in project.outline"
        :key="it.file + ':' + it.line + ':' + it.kind"
        :class="{ current: i === current, float: it.kind !== 'section' }"
        :style="{
          paddingLeft: 6 + (it.kind === 'section' ? Math.max(it.level, 0) : 4) * 10 + 'px',
        }"
        :aria-current="i === current ? 'location' : undefined"
        tabindex="0"
        @click="go(i)"
        @keydown.enter="go(i)"
      >
        <span class="glyph"><AppIcon :name="ICON[it.kind]" :size="14" /></span>
        {{ it.title }}
      </li>
    </ul>
  </section>
</template>

<style scoped>
h2 {
  margin: var(--space-3) 0 var(--space-2);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
}
ul {
  margin: 0;
  padding: 0;
  list-style: none;
}
li {
  padding-top: 3px;
  padding-bottom: 3px;
  padding-right: 6px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  cursor: pointer;
  border-radius: var(--radius-sm);
}
li:hover {
  background: var(--bg-hover);
}
li.current {
  background: var(--bg-hover);
  color: var(--accent);
  font-weight: 600;
}
li.float {
  color: var(--text-muted);
}
.glyph {
  display: inline-block;
  width: 14px;
}
.muted {
  margin: 0;
  color: var(--text-muted);
}
</style>
