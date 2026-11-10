<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useNotificationsStore } from '../stores/notifications'
import { useSettingsStore } from '../stores/settings'
import { useTexInstallStore } from '../stores/texInstall'
import { formatBytes, listResources, removeResource, type Resource } from '../tex/api'

const { t } = useI18n()
const settings = useSettingsStore()
const notifications = useNotificationsStore()
const install = useTexInstallStore()

const resources = ref<Resource[]>([])
const loading = ref(false)
const confirming = ref<string | null>(null)
const removing = ref<string | null>(null)

const total = computed(() => resources.value.reduce((sum, r) => sum + r.sizeBytes, 0))
const busy = computed(() => install.status === 'running')

function title(r: Resource): string {
  return t(`resources.kind.${r.kind}`, { year: r.year ?? '' })
}

async function refresh() {
  loading.value = true
  try {
    resources.value = await listResources()
  } catch {
    notifications.notify('error', t('resources.listFailed'))
  } finally {
    loading.value = false
  }
}

/** A árvore ativa é a que contém o `texBinDir` das configurações. */
function isActive(r: Resource): boolean {
  const bin = settings.effective.texBinDir
  if (r.kind !== 'texTree' || settings.effective.texSource !== 'managed' || !bin) return false
  const norm = (p: string) => p.replaceAll('\\', '/').toLowerCase()
  return norm(bin).startsWith(norm(r.path) + '/')
}

async function remove(r: Resource) {
  if (confirming.value !== r.id) {
    confirming.value = r.id
    return
  }
  confirming.value = null
  removing.value = r.id
  try {
    const wasActive = isActive(r)
    await removeResource(r.id)
    // Sem a árvore ativa não há TeX: o assistente volta a abrir na próxima execução.
    if (wasActive) await settings.update({ texSource: 'unset' })
    notifications.notify('success', t('resources.removed', { name: title(r) }))
  } catch {
    notifications.notify('error', t('resources.removeFailed', { name: title(r) }))
  } finally {
    removing.value = null
    await refresh()
  }
}

onMounted(refresh)
</script>

<template>
  <section class="resources" :aria-label="t('resources.title')">
    <h2>
      {{ t('resources.title') }}
      <button class="link" :disabled="loading" @click="refresh">
        {{ t('resources.refresh') }}
      </button>
    </h2>
    <p class="hint">{{ t('resources.hint') }}</p>
    <p v-if="!resources.length" class="hint">
      {{ loading ? t('resources.loading') : t('resources.empty') }}
    </p>
    <ul v-else>
      <li v-for="r in resources" :key="r.id">
        <span class="name">
          {{ title(r) }}
          <small v-if="isActive(r)" class="badge">{{ t('resources.inUse') }}</small>
        </span>
        <span class="size">{{ formatBytes(r.sizeBytes) }}</span>
        <small class="path" :title="r.path">{{ r.path }}</small>
        <button
          class="remove"
          :class="{ confirm: confirming === r.id }"
          :disabled="busy || removing === r.id"
          @click="remove(r)"
          @blur="confirming = null"
        >
          {{
            removing === r.id
              ? t('resources.removing')
              : confirming === r.id
                ? t('resources.confirm')
                : t('resources.remove')
          }}
        </button>
      </li>
    </ul>
    <p v-if="resources.length" class="total">
      {{ t('resources.total', { size: formatBytes(total) }) }}
    </p>
    <p v-if="busy" class="hint">{{ t('resources.busy') }}</p>
  </section>
</template>

<style scoped>
h2 {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.hint {
  color: var(--text-muted);
  margin: 0 0 var(--space-2);
}
ul {
  margin: 0;
  padding: 0;
  list-style: none;
}
li {
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: 2px var(--space-3);
  align-items: center;
  padding: var(--space-2) 0;
  border-bottom: 1px solid var(--border);
}
.size {
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
}
.path {
  grid-column: 1 / 3;
  color: var(--text-muted);
  font-family: var(--font-mono);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.remove {
  grid-column: 3;
  grid-row: 1 / 3;
  padding: 3px var(--space-3);
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.remove.confirm {
  border-color: var(--danger);
  color: var(--danger);
}
.remove:disabled {
  opacity: 0.5;
  cursor: default;
}
.badge {
  margin-left: var(--space-2);
  padding: 0 6px;
  border-radius: 8px;
  background: var(--accent);
  color: var(--accent-contrast);
}
.total {
  margin: var(--space-2) 0 0;
  text-align: right;
  color: var(--text-muted);
}
.link {
  background: none;
  border: 0;
  color: var(--accent);
  cursor: pointer;
  text-transform: none;
}
</style>
