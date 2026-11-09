<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  allCommands,
  effectiveBinding,
  formatBinding,
  fuzzyScore,
  type Command,
} from '../commands/registry'
import { useSettingsStore } from '../stores/settings'
import { useUiStore } from '../stores/ui'

const { t } = useI18n()
const ui = useUiStore()
const settings = useSettingsStore()

const RECENT_KEY = 'texlocal.palette.recent.v1'
const query = ref('')
const active = ref(0)
const input = ref<HTMLInputElement | null>(null)

function loadRecent(): string[] {
  try {
    return JSON.parse(localStorage.getItem(RECENT_KEY) ?? '[]') as string[]
  } catch {
    return []
  }
}
const recent = ref<string[]>(loadRecent())

interface Row {
  command: Command
  title: string
  shortcut: string
}

const rows = computed<Row[]>(() => {
  const all = allCommands().map((command) => {
    const b = effectiveBinding(command, settings.effective.keybindings)
    return { command, title: t(command.titleKey), shortcut: b ? formatBinding(b) : '' }
  })
  if (!query.value.trim()) {
    const order = (id: string) => {
      const i = recent.value.indexOf(id)
      return i === -1 ? Infinity : i
    }
    return [...all].sort((a, b) => order(a.command.id) - order(b.command.id))
  }
  return all
    .map((r) => ({ r, s: fuzzyScore(query.value, r.title) }))
    .filter((x): x is { r: Row; s: number } => x.s !== null)
    .sort((a, b) => b.s - a.s)
    .map((x) => x.r)
})

watch(
  () => ui.paletteMode,
  async (mode) => {
    if (mode) {
      query.value = ''
      active.value = 0
      await nextTick()
      input.value?.focus()
    }
  },
)
watch(query, () => (active.value = 0))

function close() {
  ui.paletteMode = null
}

function run(row: Row) {
  recent.value = [row.command.id, ...recent.value.filter((id) => id !== row.command.id)].slice(0, 8)
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(recent.value))
  } catch {
    // ignora
  }
  close()
  row.command.run()
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    active.value = Math.min(active.value + 1, rows.value.length - 1)
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    active.value = Math.max(active.value - 1, 0)
  } else if (e.key === 'Enter' && ui.paletteMode === 'commands') {
    const row = rows.value[active.value]
    if (row) run(row)
  } else if (e.key === 'Escape') {
    close()
  }
}
</script>

<template>
  <div v-if="ui.paletteMode" class="overlay" @mousedown.self="close">
    <div class="palette" role="dialog" aria-modal="true" @keydown="onKey">
      <input
        ref="input"
        v-model="query"
        role="combobox"
        aria-expanded="true"
        aria-controls="palette-list"
        :placeholder="
          ui.paletteMode === 'files'
            ? t('palette.filesPlaceholder')
            : t('palette.commandsPlaceholder')
        "
      />
      <template v-if="ui.paletteMode === 'commands'">
        <ul id="palette-list" role="listbox">
          <li
            v-for="(row, i) in rows"
            :key="row.command.id"
            role="option"
            :aria-selected="i === active"
            :class="{ active: i === active }"
            @mousemove="active = i"
            @click="run(row)"
          >
            <span>{{ row.title }}</span>
            <kbd v-if="row.shortcut">{{ row.shortcut }}</kbd>
          </li>
        </ul>
        <p v-if="!rows.length" class="empty">{{ t('palette.noResults') }}</p>
      </template>
      <p v-else class="empty">{{ t('palette.noFiles') }}</p>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  justify-content: center;
  align-items: flex-start;
  padding-top: 12vh;
  background: rgb(0 0 0 / 35%);
}
.palette {
  width: min(560px, 90vw);
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  overflow: hidden;
}
input {
  width: 100%;
  padding: var(--space-3);
  background: transparent;
  color: var(--text);
  border: 0;
  border-bottom: 1px solid var(--border);
  font-size: 14px;
}
ul {
  margin: 0;
  padding: var(--space-1);
  max-height: 320px;
  overflow: auto;
  list-style: none;
}
li {
  display: flex;
  justify-content: space-between;
  gap: var(--space-4);
  padding: 6px var(--space-2);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
li.active {
  background: var(--accent);
  color: var(--accent-contrast);
}
li.active kbd {
  color: inherit;
}
kbd {
  color: var(--text-muted);
  font-family: var(--font-ui);
}
.empty {
  margin: 0;
  padding: var(--space-3);
  color: var(--text-muted);
}
</style>
