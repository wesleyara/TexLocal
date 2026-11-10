<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { buildTree, flattenTree, type TreeNode } from '../project/tree'
import { pathKey } from '../project/model'
import { getCommand } from '../commands/registry'
import { baseName, pathExists, relativePath, revealPath } from '../project/api'
import { useCompileStore } from '../stores/compile'
import { useEditorStore } from '../stores/editor'
import { useNotificationsStore } from '../stores/notifications'
import { useProjectStore } from '../stores/project'
import { fileIconKey, folderIconKey } from '../icons/fileIcons'
import AppIcon from './AppIcon.vue'

const { t } = useI18n()
const project = useProjectStore()
const editor = useEditorStore()
const compile = useCompileStore()
const notifications = useNotificationsStore()

// O PDF compilado fica no `outdir` (oculto na árvore): ganha uma entrada própria em "Saída".
const pdfExists = ref(false)
watch(
  () => [compile.pdfPath, compile.pdfVersion, compile.state] as const,
  async ([path]) => {
    pdfExists.value = path ? await pathExists(path) : false
    // Terminou uma compilação: a pasta de build mudou.
    if (compile.state !== 'running') await project.refreshFiles()
  },
  { immediate: true },
)

async function revealPdf() {
  if (!compile.pdfPath) return
  try {
    await revealPath(compile.pdfPath)
  } catch (e) {
    notifications.notify('error', String(e))
  }
}

// Pastas começam recolhidas; só as do arquivo aberto na aba ativa são expandidas.
const expanded = ref(new Set<string>())
const tree = computed(() => buildTree(project.files))
const collapsed = computed(() => {
  const out = new Set<string>()
  const walk = (nodes: TreeNode[]) => {
    for (const n of nodes) {
      if (!n.dir) continue
      if (!expanded.value.has(n.path)) out.add(n.path)
      walk(n.children)
    }
  }
  walk(tree.value)
  return out
})
const rows = computed(() => flattenTree(tree.value, collapsed.value))

/** Expande as pastas que contêm o arquivo ativo. */
function revealActive() {
  const active = editor.active
  if (!active || !project.root) return
  const rel = pathKey(relativePath(project.root, active))
  if (rel.startsWith('..')) return
  const parts = rel.split('/').slice(0, -1)
  if (!parts.length) return
  const next = new Set(expanded.value)
  parts.forEach((_, i) => next.add(parts.slice(0, i + 1).join('/')))
  expanded.value = next
}
watch(() => [editor.active, project.files.length], revealActive, { immediate: true })
watch(
  () => project.root,
  () => {
    expanded.value = new Set()
    revealActive()
  },
)

function toggle(path: string) {
  const next = new Set(expanded.value)
  if (!next.delete(path)) next.add(path)
  expanded.value = next
}

function click(row: { path: string; dir: boolean }) {
  if (row.dir) toggle(row.path)
  else void editor.openFile(project.absolute(row.path))
}
</script>

<template>
  <div class="files">
    <template v-if="project.root">
      <ul role="tree" :aria-label="t('sidebar.files')">
        <li
          v-for="row in rows"
          :key="row.path"
          role="treeitem"
          :aria-expanded="row.dir ? !collapsed.has(row.path) : undefined"
          :class="{
            dir: row.dir,
            active:
              !row.dir && editor.active && editor.active.replace(/\\/g, '/').endsWith(row.path),
          }"
          :style="{ paddingLeft: 8 + row.depth * 12 + 'px' }"
          tabindex="0"
          @click="click(row)"
          @keydown.enter="click(row)"
        >
          <span class="chevron">
            <AppIcon
              v-if="row.dir"
              :name="collapsed.has(row.path) ? 'chevron-right' : 'chevron-down'"
              :size="14"
            />
          </span>
          <AppIcon
            :name="
              row.dir ? folderIconKey(row.name, !collapsed.has(row.path)) : fileIconKey(row.name)
            "
            :size="16"
          />
          <span class="label">{{ row.name }}</span>
        </li>
      </ul>
      <template v-if="pdfExists && compile.pdfPath">
        <h3 class="out-title">{{ t('sidebar.output') }}</h3>
        <ul>
          <li
            class="output"
            tabindex="0"
            :title="`${compile.pdfPath}
${t('sidebar.revealPdf')}`"
            @click="revealPdf"
            @keydown.enter="revealPdf"
          >
            <span class="chevron" />
            <AppIcon :name="fileIconKey('x.pdf')" :size="16" />
            <span class="label">{{ baseName(compile.pdfPath) }}</span>
            <AppIcon class="reveal" name="folder-open-outline" :size="14" />
          </li>
        </ul>
      </template>
    </template>
    <div v-else class="none">
      <p class="muted">{{ t('sidebar.empty') }}</p>
      <button @click="getCommand('project.open')?.run()">{{ t('project.open') }}</button>
    </div>
  </div>
</template>

<style scoped>
ul {
  margin: 0;
  padding: 0;
  list-style: none;
}
li {
  display: flex;
  align-items: center;
  gap: 6px;
  padding-top: 3px;
  padding-bottom: 3px;
  padding-right: 8px;
  white-space: nowrap;
  overflow: hidden;
  cursor: pointer;
  border-radius: var(--radius-sm);
}
li:hover {
  background: var(--bg-hover);
}
li.active {
  background: var(--bg-hover);
  color: var(--accent);
}
.out-title {
  margin: var(--space-3) 0 var(--space-1);
  padding: 0 8px;
  font-size: 11px;
  font-weight: 400;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
}
.output {
  padding-left: 8px;
}
.reveal {
  margin-left: auto;
  flex: none;
  color: var(--text-muted);
}
.chevron {
  display: inline-flex;
  width: 14px;
  flex: none;
  color: var(--text-muted);
}
.label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
.muted {
  color: var(--text-muted);
  margin: 0 0 var(--space-2);
}
button {
  padding: var(--space-1) var(--space-3);
  background: var(--bg-hover);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  cursor: pointer;
}
</style>
