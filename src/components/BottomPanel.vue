<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { relativePath } from '../project/api'
import { labelProblems } from '../project/model'
import { useEditorStore } from '../stores/editor'
import { useProjectStore } from '../stores/project'
import { useRefsStore } from '../stores/refs'

const { t } = useI18n()
const editor = useEditorStore()
const project = useProjectStore()
const refs = useRefsStore()

const problems = computed(() => (editor.active ? labelProblems(project.view, editor.active) : []))

function rel(path: string) {
  return project.root ? relativePath(project.root, path) : path
}
</script>

<template>
  <div class="bottom">
    <h2>{{ t('bottom.problems') }}</h2>
    <p v-if="!problems.length && !project.view.missing.length" class="muted">
      {{ t('bottom.noProblems') }}
    </p>
    <ul>
      <li
        v-for="(p, i) in problems"
        :key="i"
        class="row"
        :class="p.severity"
        tabindex="0"
        @click="
          editor.active &&
          editor.openFile(editor.active, { line: p.line, col: p.col, endCol: p.endCol })
        "
      >
        <span class="sev" aria-hidden="true">{{ p.severity === 'error' ? '✖' : '▲' }}</span>
        {{ t(p.messageKey, p.params) }}
        <span class="loc">{{ p.line + 1 }}:{{ p.col + 1 }}</span>
      </li>
      <li v-for="m in project.view.missing" :key="m" class="row warning">
        <span class="sev" aria-hidden="true">▲</span>
        {{ t('editor.problem.missingFile', { name: m }) }}
      </li>
    </ul>

    <template v-if="refs.name">
      <h2>
        {{ t('editor.usages', { name: refs.name }) }}
        <button class="x" :aria-label="t('settings.close')" @click="refs.clear()">×</button>
      </h2>
      <ul>
        <li
          v-if="refs.definition"
          class="row"
          tabindex="0"
          @click="editor.openFile(refs.definition.file, refs.definition)"
        >
          <strong>{{ t('editor.definition') }}</strong> {{ rel(refs.definition.file) }}:{{
            refs.definition.line + 1
          }}
        </li>
        <li
          v-for="(u, i) in refs.usages"
          :key="i"
          class="row"
          tabindex="0"
          @click="editor.openFile(u.file, u)"
        >
          {{ rel(u.file) }}:{{ u.line + 1 }}
        </li>
        <li v-if="!refs.usages.length" class="muted">{{ t('editor.noUsages') }}</li>
      </ul>
    </template>
  </div>
</template>

<style scoped>
h2 {
  margin: 0 0 var(--space-2);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
}
h2:not(:first-child) {
  margin-top: var(--space-3);
}
ul {
  margin: 0;
  padding: 0;
  list-style: none;
}
.row {
  padding: 2px var(--space-2);
  cursor: pointer;
  border-radius: var(--radius-sm);
}
.row:hover {
  background: var(--bg-hover);
}
.error .sev {
  color: var(--danger);
}
.warning .sev {
  color: var(--text-muted);
}
.loc {
  margin-left: var(--space-2);
  color: var(--text-muted);
}
.muted {
  margin: 0;
  color: var(--text-muted);
}
.x {
  margin-left: var(--space-2);
  background: none;
  border: 0;
  color: inherit;
  cursor: pointer;
}
</style>
