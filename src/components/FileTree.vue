<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { buildTree, flattenTree } from '../project/tree'
import { getCommand } from '../commands/registry'
import { useEditorStore } from '../stores/editor'
import { useProjectStore } from '../stores/project'
import { fileIconKey, folderIconKey } from '../icons/fileIcons'
import AppIcon from './AppIcon.vue'

const { t } = useI18n()
const project = useProjectStore()
const editor = useEditorStore()

const collapsed = ref(new Set<string>())
const rows = computed(() => flattenTree(buildTree(project.files), collapsed.value))

function toggle(path: string) {
  const next = new Set(collapsed.value)
  if (!next.delete(path)) next.add(path)
  collapsed.value = next
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
