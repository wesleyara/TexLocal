<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useEditorStore } from '../stores/editor'
import { useProjectStore } from '../stores/project'
import CodeEditor from './CodeEditor.vue'
import MediaViewer from './MediaViewer.vue'
import TabBar from './TabBar.vue'

const { t } = useI18n()
const editor = useEditorStore()
const project = useProjectStore()
</script>

<template>
  <div class="editor-pane">
    <TabBar />
    <MediaViewer
      v-if="editor.activeTab?.media"
      :key="editor.activeTab.path"
      :path="editor.activeTab.path"
      :kind="editor.activeTab.media"
    />
    <CodeEditor v-show="!editor.activeTab?.media" />
    <div v-if="!editor.active" class="empty">
      <p class="muted">{{ project.root ? t('editor.noFile') : t('editor.placeholder') }}</p>
      <button v-if="!project.root" class="primary" @click="project.openFolder()">
        {{ t('project.open') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.editor-pane {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
}
.empty {
  position: absolute;
  inset: 32px 0 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-3);
}
.muted {
  margin: 0;
  color: var(--text-muted);
}
.primary {
  padding: var(--space-2) var(--space-4);
  background: var(--accent);
  color: var(--accent-contrast);
  border: 0;
  border-radius: var(--radius);
  cursor: pointer;
}
</style>
