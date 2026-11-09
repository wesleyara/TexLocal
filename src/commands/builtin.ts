import { useSettingsStore } from '../stores/settings'
import { useUiStore } from '../stores/ui'
import { registerCommand } from './registry'

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
    id: 'tex.wizard',
    titleKey: 'cmd.openTexWizard',
    run: () => (ui().wizardOpen = true),
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
