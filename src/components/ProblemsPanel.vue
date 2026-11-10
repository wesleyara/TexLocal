<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  diagnosticFiles,
  explanationKey,
  filterDiagnostics,
  resolveFile,
  type KindFilter,
} from '../compile/model'
import type { Diagnostic } from '../compile/api'
import { relativePath } from '../project/api'
import { labelProblems } from '../project/model'
import { useCompileStore } from '../stores/compile'
import { useEditorStore } from '../stores/editor'
import { usePackagesStore } from '../stores/packages'
import { useProjectStore } from '../stores/project'
import { useRefsStore } from '../stores/refs'
import AppIcon from './AppIcon.vue'

const { t, te } = useI18n()
const editor = useEditorStore()
const project = useProjectStore()
const refs = useRefsStore()
const compile = useCompileStore()
const packages = usePackagesStore()

const kind = ref<KindFilter>('all')
const file = ref<string | null>(null)

const files = computed(() => diagnosticFiles(compile.diagnostics))
const shown = computed(() =>
  filterDiagnostics(compile.diagnostics, {
    kind: kind.value,
    showBoxes: compile.showBoxes,
    file: file.value,
  }),
)
const labelIssues = computed(() =>
  editor.active ? labelProblems(project.view, editor.active) : [],
)
const hasCompiled = computed(() => compile.outcome !== null)

function rel(path: string) {
  return project.root ? relativePath(project.root, path) : path
}

function friendly(d: Diagnostic): string | null {
  const key = explanationKey(d)
  return key && te(key) ? t(key) : null
}

function open(d: Diagnostic) {
  if (!d.file || !project.root) return
  void editor.openFile(resolveFile(project.root, d.file), {
    line: Math.max((d.line ?? 1) - 1, 0),
    col: 0,
  })
}
</script>

<template>
  <div class="problems">
    <div v-if="packages.suggestion" class="banner" role="alert">
      <AppIcon name="download" :size="16" />
      <span class="text">
        <strong>{{
          t('packages.missing', { names: packages.names }, packages.suggestion.packages.length)
        }}</strong>
        <small>{{ t('packages.files', { files: packages.suggestion.files.join(', ') }) }}</small>
      </span>
      <button class="primary" :disabled="packages.installing" @click="packages.install()">
        {{ packages.installing ? t('packages.installingShort') : t('packages.install') }}
      </button>
      <button :disabled="packages.installing" @click="packages.dismiss()">
        {{ t('packages.ignore') }}
      </button>
    </div>
    <p v-if="packages.error" class="banner-note error" role="alert">{{ packages.error }}</p>
    <p v-if="packages.unresolved.length" class="banner-note">
      {{ t('packages.unresolved', { files: packages.unresolved.join(', ') }) }}
    </p>

    <div class="filters" role="group" :aria-label="t('compile.filters')">
      <select v-model="kind" :aria-label="t('compile.filter.kind')">
        <option value="all">{{ t('compile.filter.all') }}</option>
        <option value="errors">{{ t('compile.filter.errors') }}</option>
        <option value="warnings">{{ t('compile.filter.warnings') }}</option>
      </select>
      <select v-model="file" :aria-label="t('compile.filter.file')">
        <option :value="null">{{ t('compile.filter.allFiles') }}</option>
        <option v-for="f in files" :key="f" :value="f">{{ f }}</option>
      </select>
      <label class="check">
        <input v-model="compile.showBoxes" type="checkbox" />
        {{ t('compile.filter.boxes') }}
      </label>
    </div>

    <p v-if="!hasCompiled && !labelIssues.length && !project.view.missing.length" class="muted">
      {{ t('compile.notCompiled') }}
    </p>
    <p
      v-else-if="
        hasCompiled && !shown.length && !labelIssues.length && !project.view.missing.length
      "
      class="muted"
    >
      {{ t('bottom.noProblems') }}
    </p>

    <ul>
      <li
        v-for="(d, i) in shown"
        :key="`c${i}`"
        class="row"
        :class="d.severity"
        tabindex="0"
        @click="open(d)"
        @keydown.enter="open(d)"
      >
        <span class="sev"
          ><AppIcon :name="d.severity === 'error' ? 'alert-circle' : 'alert'" :size="15"
        /></span>
        <span class="msg">
          {{ friendly(d) ?? d.message }}
          <small v-if="friendly(d)" class="raw">{{ d.message }}</small>
          <small v-if="d.context" class="raw code">{{ d.context }}</small>
        </span>
        <span v-if="d.file" class="loc">{{ d.file }}{{ d.line ? `:${d.line}` : '' }}</span>
      </li>

      <li
        v-for="(p, i) in labelIssues"
        :key="`l${i}`"
        class="row"
        :class="p.severity"
        tabindex="0"
        @click="
          editor.active &&
          editor.openFile(editor.active, { line: p.line, col: p.col, endCol: p.endCol })
        "
      >
        <span class="sev"
          ><AppIcon :name="p.severity === 'error' ? 'alert-circle' : 'alert'" :size="15"
        /></span>
        <span class="msg">{{ t(p.messageKey, p.params) }}</span>
        <span class="loc">{{ p.line + 1 }}:{{ p.col + 1 }}</span>
      </li>
      <li v-for="m in project.view.missing" :key="m" class="row warning">
        <span class="sev"><AppIcon name="alert" :size="15" /></span>
        <span class="msg">{{ t('editor.problem.missingFile', { name: m }) }}</span>
      </li>
    </ul>

    <template v-if="refs.name">
      <h2>
        {{ t('editor.usages', { name: refs.name }) }}
        <button class="x" :aria-label="t('settings.close')" @click="refs.clear()">
          <AppIcon name="close" :size="14" />
        </button>
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
  margin: var(--space-3) 0 var(--space-2);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
}
.banner {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  margin-bottom: var(--space-2);
  background: var(--bg-elevated);
  border: 1px solid var(--accent);
  border-radius: var(--radius);
}
.banner .text {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
}
.banner small,
.banner-note {
  color: var(--text-muted);
}
.banner-note {
  margin: 0 0 var(--space-2);
}
.banner-note.error {
  color: var(--danger);
}
.banner button {
  padding: 3px var(--space-3);
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.banner button.primary {
  background: var(--accent);
  color: var(--accent-contrast);
  border-color: var(--accent);
}
.banner button:disabled {
  opacity: 0.6;
  cursor: default;
}
.filters {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  margin-bottom: var(--space-2);
}
.filters select {
  max-width: 180px;
  background: var(--bg-elevated);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 1px var(--space-2);
}
.check {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  color: var(--text-muted);
}
ul {
  margin: 0;
  padding: 0;
  list-style: none;
}
.row {
  display: flex;
  align-items: baseline;
  gap: var(--space-2);
  padding: 2px var(--space-2);
  cursor: pointer;
  border-radius: var(--radius-sm);
}
.row:hover,
.row:focus-visible {
  background: var(--bg-hover);
}
.msg {
  flex: 1;
  min-width: 0;
}
.raw {
  display: block;
  color: var(--text-muted);
}
.code {
  font-family: var(--font-mono, monospace);
}
.error .sev {
  color: var(--danger);
}
.warning .sev {
  color: var(--text-muted);
}
.loc {
  flex: none;
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
