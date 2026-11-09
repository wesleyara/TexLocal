<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { allCommands, effectiveBinding, matchesEvent } from '../commands/registry'
import { useSettingsStore } from '../stores/settings'
import { useUiStore } from '../stores/ui'
import ActivityBar from './ActivityBar.vue'
import CommandPalette from './CommandPalette.vue'
import SettingsDialog from './SettingsDialog.vue'
import SplitHandle from './SplitHandle.vue'
import StatusBar from './StatusBar.vue'
import TitleBar from './TitleBar.vue'
import ToastHost from './ToastHost.vue'

const { t } = useI18n()
const ui = useUiStore()
const settings = useSettingsStore()

function onKeydown(e: KeyboardEvent) {
  if (e.defaultPrevented) return
  if (e.key === 'Escape' && (ui.paletteMode || ui.settingsOpen)) {
    e.preventDefault()
    ui.paletteMode = null
    ui.settingsOpen = false
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
          <h2>{{ t('sidebar.files') }}</h2>
          <p class="muted">{{ t('sidebar.empty') }}</p>
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
            <h2>{{ t('editor.title') }}</h2>
            <p class="muted">{{ t('editor.placeholder') }}</p>
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
                ? { width: ui.layout.pdfWidth + 'px', flex: 'none' }
                : {}
            "
          >
            <h2>{{ t('pdf.title') }}</h2>
            <p class="muted">{{ t('pdf.placeholder') }}</p>
          </section>
        </div>

        <template v-if="ui.layout.bottomVisible">
          <SplitHandle
            orientation="horizontal"
            :label="t('layout.bottomResize')"
            @resize="(d) => ui.resize('bottomHeight', ui.layout.bottomHeight - d)"
          />
          <section class="bottom" :style="{ height: ui.layout.bottomHeight + 'px' }">
            <h2>{{ t('bottom.problems') }}</h2>
            <p class="muted">{{ t('bottom.noProblems') }}</p>
          </section>
        </template>
      </div>
    </div>
    <StatusBar />
    <CommandPalette />
    <SettingsDialog />
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
.pdf {
  background: var(--bg-panel);
}
.bottom {
  flex: none;
  padding: var(--space-3);
  background: var(--bg-panel);
  overflow: auto;
}
h2 {
  margin: 0 0 var(--space-2);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-muted);
}
.muted {
  margin: 0;
  color: var(--text-muted);
}
</style>
