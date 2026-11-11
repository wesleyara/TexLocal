<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { commandTitle, formatBinding, effectiveBinding, getCommand } from '../commands/registry'
import { useSettingsStore } from '../stores/settings'

const { t } = useI18n()
const settings = useSettingsStore()

interface Menu {
  labelKey: string
  items: string[]
}

const menus: Menu[] = [
  { labelKey: 'menu.file', items: ['project.open', 'palette.files', 'settings.open'] },
  {
    labelKey: 'menu.view',
    items: [
      'palette.commands',
      'view.toggleSidebar',
      'view.toggleBottom',
      'view.modeSplit',
      'view.modeEditor',
      'view.modePdf',
      'theme.toggle',
    ],
  },
]

const entries = computed(() =>
  menus.map((m) => ({
    ...m,
    commands: m.items
      .map((id) => getCommand(id))
      .filter((c): c is NonNullable<typeof c> => c !== undefined)
      .map((c) => {
        const b = effectiveBinding(c, settings.effective.keybindings)
        return { command: c, shortcut: b ? formatBinding(b) : '' }
      }),
  })),
)
</script>

<template>
  <header class="titlebar">
    <strong class="brand">{{ t('app.name') }}</strong>
    <nav class="menus">
      <details v-for="m in entries" :key="m.labelKey" class="menu">
        <summary>{{ t(m.labelKey) }}</summary>
        <ul role="menu">
          <li v-for="c in m.commands" :key="c.command.id" role="none">
            <button
              role="menuitem"
              @click="
                (e) => {
                  c.command.run()
                  ;(e.currentTarget as HTMLElement).closest('details')?.removeAttribute('open')
                }
              "
            >
              <span>{{ commandTitle(c.command, t) }}</span>
              <kbd v-if="c.shortcut">{{ c.shortcut }}</kbd>
            </button>
          </li>
        </ul>
      </details>
    </nav>
  </header>
</template>

<style scoped>
.titlebar {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  height: 32px;
  padding: 0 var(--space-3);
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border);
  flex: none;
}
.brand {
  font-size: 12px;
}
.menus {
  display: flex;
  gap: var(--space-1);
}
.menu {
  position: relative;
}
summary {
  list-style: none;
  cursor: pointer;
  padding: 4px var(--space-2);
  border-radius: var(--radius-sm);
}
summary::-webkit-details-marker {
  display: none;
}
summary:hover,
details[open] > summary {
  background: var(--bg-hover);
}
ul {
  position: absolute;
  top: 100%;
  left: 0;
  z-index: 50;
  min-width: 280px;
  margin: 2px 0 0;
  padding: var(--space-1);
  list-style: none;
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
}
li button {
  display: flex;
  justify-content: space-between;
  gap: var(--space-4);
  width: 100%;
  padding: 6px var(--space-2);
  background: none;
  border: 0;
  border-radius: var(--radius-sm);
  text-align: left;
  cursor: pointer;
}
li button:hover {
  background: var(--bg-hover);
}
kbd {
  color: var(--text-muted);
  font-family: var(--font-ui);
}
</style>
