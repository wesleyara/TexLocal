<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { allCommands, effectiveBinding, getCommand, matchesEvent } from '../commands/registry'
import { useSettingsStore } from '../stores/settings'
import { useUiStore } from '../stores/ui'
import ActivityBar from './ActivityBar.vue'
import AppIcon from './AppIcon.vue'
import BottomPanel from './BottomPanel.vue'
import CommandPalette from './CommandPalette.vue'
import EditorPane from './EditorPane.vue'
import FileTree from './FileTree.vue'
import OutlinePanel from './OutlinePanel.vue'
import PdfViewer from './PdfViewer.vue'
import RenameDialog from './RenameDialog.vue'
import SearchPanel from './SearchPanel.vue'
import SettingsDialog from './SettingsDialog.vue'
import SplitHandle from './SplitHandle.vue'
import StatusBar from './StatusBar.vue'
import TexWizard from './TexWizard.vue'
import TitleBar from './TitleBar.vue'
import ToastHost from './ToastHost.vue'

const { t } = useI18n()
const ui = useUiStore()
const settings = useSettingsStore()

function onKeydown(e: KeyboardEvent) {
  if (e.defaultPrevented) return
  if (e.key === 'Escape' && (ui.paletteMode || ui.settingsOpen || ui.wizardOpen)) {
    e.preventDefault()
    ui.paletteMode = null
    ui.settingsOpen = false
    ui.wizardOpen = false
    return
  }
  // "Ctrl++" chega como key "+", que não cabe na sintaxe de atalhos ("mod+shift++").
  if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key === '+') {
    e.preventDefault()
    getCommand('view.zoomIn')?.run()
    return
  }
  const overrides = settings.effective.keybindings
  for (const command of allCommands()) {
    const binding = effectiveBinding(command, overrides)
    if (binding && matchesEvent(binding, e)) {
      e.preventDefault()
      command.run()
      return
    }
  }
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <div class="shell">
    <TitleBar />
    <div class="workbench">
      <ActivityBar />

      <template v-if="ui.layout.sidebarVisible">
        <aside class="sidebar" :style="{ width: ui.layout.sidebarWidth + 'px' }">
          <template v-if="ui.sidebarPanel === 'files'">
            <div class="head">
              <h2>{{ t('sidebar.files') }}</h2>
              <button
                class="icon-btn"
                :title="t('cmd.openFolder')"
                :aria-label="t('cmd.openFolder')"
                @click="getCommand('project.open')?.run()"
              >
                <AppIcon name="folder-open-outline" :size="16" />
              </button>
            </div>
            <FileTree />
            <OutlinePanel />
          </template>
          <SearchPanel v-else />
        </aside>
        <SplitHandle
          orientation="vertical"
          :label="t('layout.sidebarResize')"
          @resize="(d) => ui.resize('sidebarWidth', ui.layout.sidebarWidth + d)"
        />
      </template>

      <div class="main">
        <div class="panes">
          <section v-if="ui.layout.viewMode !== 'pdf'" class="pane editor">
            <EditorPane />
          </section>

          <SplitHandle
            v-if="ui.layout.viewMode === 'split'"
            orientation="vertical"
            :label="t('layout.pdfResize')"
            @resize="(d) => ui.resize('pdfWidth', ui.layout.pdfWidth - d)"
          />

          <section
            v-if="ui.layout.viewMode !== 'editor'"
            class="pane pdf"
            :style="
              ui.layout.viewMode === 'split'
                ? { width: ui.layout.pdfWidth + 'px', flex: 'none', maxWidth: 'calc(100% - 300px)' }
                : {}
            "
          >
            <PdfViewer />
          </section>
        </div>

        <template v-if="ui.layout.bottomVisible">
          <SplitHandle
            orientation="horizontal"
            :label="t('layout.bottomResize')"
            @resize="(d) => ui.resize('bottomHeight', ui.layout.bottomHeight - d)"
          />
          <section class="bottom" :style="{ height: ui.layout.bottomHeight + 'px' }">
            <BottomPanel />
          </section>
        </template>
      </div>
    </div>
    <StatusBar />
    <CommandPalette />
    <SettingsDialog />
    <TexWizard />
    <RenameDialog />
    <ToastHost />
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 900px;
  overflow: hidden;
}
.workbench {
  display: flex;
  flex: 1;
  min-height: 0;
}
.sidebar {
  flex: none;
  padding: var(--space-3);
  background: var(--bg-panel);
  overflow: auto;
}
.main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
}
.panes {
  display: flex;
  flex: 1;
  min-height: 0;
}
.pane {
  flex: 1;
  min-width: 0;
  padding: var(--space-3);
  overflow: auto;
}
.pane.editor {
  min-width: 280px;
  display: flex;
  padding: 0;
  overflow: hidden;
}
.pdf {
  display: flex;
  padding: 0;
  overflow: hidden;
  background: var(--bg-panel);
}
.bottom {
  flex: none;
  padding: var(--space-3);
  background: var(--bg-panel);
  overflow: auto;
}
h2 {
  margin: 0;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-2);
}
.icon-btn {
  display: inline-flex;
  padding: 3px;
  background: none;
  border: 0;
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  cursor: pointer;
}
.icon-btn:hover {
  background: var(--bg-hover);
  color: var(--text);
}
.muted {
  margin: 0;
  color: var(--text-muted);
}
</style>
