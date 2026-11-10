import { pickFolder, revealPath } from '../project/api'
import { getActiveView } from '../editor/actions'
import { useCompileStore } from '../stores/compile'
import { useEditorStore } from '../stores/editor'
import { useProjectStore } from '../stores/project'
import { useSettingsStore } from '../stores/settings'
import { useUiStore } from '../stores/ui'
import { i18n } from '../i18n'
import { useNotificationsStore } from '../stores/notifications'
import { clampZoom } from '../settings/zoom'
import { registerCommand } from './registry'

async function openFolder() {
  const project = useProjectStore()
  const editor = useEditorStore()
  const picked = await pickFolder()
  if (!picked) return
  editor.reset()
  await project.open(picked)
  await editor.restoreSession()
}

async function openSettingsJson() {
  const path = await useSettingsStore().filePath()
  if (!path) {
    useNotificationsStore().notify('info', i18n.global.t('settings.jsonDesktopOnly'))
    return
  }
  await useEditorStore().openFile(path)
}

function cycleTab(delta: number) {
  const e = useEditorStore()
  if (!e.tabs.length) return
  const i = e.tabs.findIndex((t) => t.path === e.active)
  e.activate(e.tabs[(i + delta + e.tabs.length) % e.tabs.length].path)
}

/** Registro central: menus, atalhos e command palette usam esta mesma lista. */
export function registerBuiltinCommands(): void {
  const ui = () => useUiStore()

  registerCommand({
    id: 'palette.commands',
    titleKey: 'cmd.palette',
    keybinding: 'Mod+Shift+P',
    run: () => (ui().paletteMode = 'commands'),
  })
  registerCommand({
    id: 'palette.files',
    titleKey: 'cmd.quickOpen',
    keybinding: 'Mod+P',
    run: () => (ui().paletteMode = 'files'),
  })
  registerCommand({
    id: 'view.toggleSidebar',
    titleKey: 'cmd.toggleSidebar',
    keybinding: 'Mod+B',
    run: () => ui().toggleSidebar(),
  })
  registerCommand({
    id: 'view.toggleBottom',
    titleKey: 'cmd.toggleBottom',
    keybinding: 'Mod+J',
    run: () => ui().toggleBottom(),
  })
  registerCommand({
    id: 'view.modeSplit',
    titleKey: 'cmd.modeSplit',
    run: () => ui().setViewMode('split'),
  })
  registerCommand({
    id: 'view.modeEditor',
    titleKey: 'cmd.modeEditor',
    run: () => ui().setViewMode('editor'),
  })
  registerCommand({
    id: 'view.modePdf',
    titleKey: 'cmd.modePdf',
    run: () => ui().setViewMode('pdf'),
  })
  registerCommand({
    id: 'settings.open',
    titleKey: 'cmd.openSettings',
    keybinding: 'Mod+,',
    run: () => (ui().settingsOpen = true),
  })
  registerCommand({
    id: 'settings.openJson',
    titleKey: 'cmd.openSettingsJson',
    keybinding: 'Mod+Shift+,',
    run: () => void openSettingsJson(),
  })
  const zoomBy = (delta: number) => {
    const s = useSettingsStore()
    void s.update({ zoomLevel: clampZoom(s.effective.zoomLevel + delta) })
  }
  registerCommand({
    id: 'view.zoomIn',
    titleKey: 'cmd.zoomIn',
    keybinding: 'Mod+=',
    run: () => zoomBy(1),
  })
  registerCommand({
    id: 'view.zoomOut',
    titleKey: 'cmd.zoomOut',
    keybinding: 'Mod+-',
    run: () => zoomBy(-1),
  })
  registerCommand({
    id: 'view.zoomReset',
    titleKey: 'cmd.zoomReset',
    keybinding: 'Mod+0',
    run: () => void useSettingsStore().update({ zoomLevel: 0 }),
  })
  registerCommand({
    id: 'tex.wizard',
    titleKey: 'cmd.openTexWizard',
    run: () => (ui().wizardOpen = true),
  })
  registerCommand({
    id: 'project.open',
    titleKey: 'cmd.openFolder',
    keybinding: 'Mod+O',
    run: () => void openFolder(),
  })
  registerCommand({
    id: 'editor.save',
    titleKey: 'cmd.save',
    keybinding: 'Mod+S',
    run: () => {
      const path = useEditorStore().active
      if (!path) return
      void useEditorStore()
        .save(path)
        .then((ok) => {
          if (ok) void useCompileStore().afterExplicitSave(path)
        })
    },
  })
  registerCommand({
    id: 'compile.run',
    titleKey: 'cmd.compile',
    keybinding: 'Mod+Alt+B',
    run: () => void useCompileStore().compile({ manual: true }),
  })
  registerCommand({
    id: 'compile.revealPdf',
    titleKey: 'cmd.revealPdf',
    run: () => {
      const path = useCompileStore().pdfPath
      if (path)
        void revealPath(path).catch((e) => useNotificationsStore().notify('error', String(e)))
    },
  })
  registerCommand({
    id: 'compile.cancel',
    titleKey: 'cmd.cancelCompile',
    run: () => void useCompileStore().cancel(),
  })
  registerCommand({
    id: 'compile.toggleAuto',
    titleKey: 'cmd.toggleAutoCompile',
    run: () => void useCompileStore().toggleAuto(),
  })
  registerCommand({
    id: 'compile.toggleShellEscape',
    titleKey: 'cmd.toggleShellEscape',
    run: () => {
      const compile = useCompileStore()
      void compile.setShellEscape(!compile.shellEscape)
    },
  })
  registerCommand({
    id: 'compile.syncToPdf',
    titleKey: 'cmd.syncToPdf',
    keybinding: 'Mod+Alt+J',
    run: () => void useCompileStore().syncToPdf(),
  })
  registerCommand({
    id: 'compile.toggleBoxWarnings',
    titleKey: 'cmd.toggleBoxWarnings',
    run: () => {
      const compile = useCompileStore()
      compile.showBoxes = !compile.showBoxes
    },
  })
  registerCommand({
    id: 'editor.saveAll',
    titleKey: 'cmd.saveAll',
    keybinding: 'Mod+Alt+S',
    run: () => void useEditorStore().saveAll(),
  })
  registerCommand({
    id: 'editor.closeTab',
    titleKey: 'cmd.closeTab',
    keybinding: 'Mod+W',
    run: () => {
      const path = useEditorStore().active
      if (path) void useEditorStore().close(path)
    },
  })
  registerCommand({
    id: 'editor.nextTab',
    titleKey: 'cmd.nextTab',
    keybinding: 'Mod+PageDown',
    run: () => cycleTab(1),
  })
  registerCommand({
    id: 'editor.prevTab',
    titleKey: 'cmd.prevTab',
    keybinding: 'Mod+PageUp',
    run: () => cycleTab(-1),
  })
  registerCommand({
    id: 'editor.toggleWrap',
    titleKey: 'cmd.toggleWrap',
    keybinding: 'Alt+Z',
    run: () => {
      const e = useEditorStore()
      e.setPrefs({ wrap: !e.prefs.wrap })
    },
  })
  registerCommand({
    id: 'editor.cycleAutosave',
    titleKey: 'cmd.cycleAutosave',
    run: () => {
      const e = useEditorStore()
      const order = ['both', 'delay', 'blur', 'off'] as const
      e.setPrefs({ autosave: order[(order.indexOf(e.prefs.autosave) + 1) % order.length] })
      useNotificationsStore().notify('info', i18n.global.t(`editor.autosave.${e.prefs.autosave}`))
    },
  })
  registerCommand({
    id: 'editor.toggleEol',
    titleKey: 'cmd.toggleEol',
    run: () => {
      const e = useEditorStore()
      const tab = e.activeTab
      if (tab) e.setEol(tab.path, tab.eol === 'lf' ? 'crlf' : 'lf')
    },
  })
  registerCommand({
    id: 'editor.gotoLine',
    titleKey: 'cmd.gotoLine',
    keybinding: 'Mod+G',
    run: () => {
      const view = getActiveView()
      if (view) void import('@codemirror/search').then((m) => m.gotoLine(view))
    },
  })
  registerCommand({
    id: 'search.project',
    titleKey: 'cmd.searchProject',
    keybinding: 'Mod+Shift+F',
    run: () => useUiStore().showSidebarPanel('search'),
  })
  registerCommand({
    id: 'theme.toggle',
    titleKey: 'cmd.toggleTheme',
    run: () => {
      const s = useSettingsStore()
      void s.update({ theme: s.effective.theme === 'light' ? 'dark' : 'light' })
    },
  })
}
