<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { searchLog } from '../compile/model'
import { useCompileStore } from '../stores/compile'

const { t } = useI18n()
const compile = useCompileStore()

const source = ref<'log' | 'output'>('log')
const query = ref('')
const current = ref(0)
const body = ref<HTMLElement | null>(null)

const text = computed(() => {
  const o = compile.outcome
  if (!o) return ''
  return source.value === 'log' ? o.log || o.output : o.output
})
const lines = computed(() => text.value.split('\n'))
const hits = computed(() => searchLog(text.value, query.value))

/** Trechos de uma linha, marcando as ocorrências da busca. */
function segments(lineNo: number, line: string) {
  const own = hits.value.map((h, index) => ({ h, index })).filter(({ h }) => h[0] === lineNo)
  if (!own.length) return [{ text: line, hit: -1 }]
  const out: Array<{ text: string; hit: number }> = []
  let from = 0
  for (const { h, index } of own) {
    if (h[1] > from) out.push({ text: line.slice(from, h[1]), hit: -1 })
    out.push({ text: line.slice(h[1], h[1] + query.value.length), hit: index })
    from = h[1] + query.value.length
  }
  if (from < line.length) out.push({ text: line.slice(from), hit: -1 })
  return out
}

function go(delta: number) {
  if (!hits.value.length) return
  current.value = (current.value + delta + hits.value.length) % hits.value.length
  void nextTick(() =>
    body.value?.querySelector('mark.current')?.scrollIntoView({ block: 'center' }),
  )
}

watch(query, () => {
  current.value = 0
  go(0)
})
</script>

<template>
  <div class="log">
    <div class="bar">
      <input
        v-model="query"
        type="search"
        :placeholder="t('compile.log.search')"
        :aria-label="t('compile.log.search')"
        @keydown.enter.exact.prevent="go(1)"
        @keydown.enter.shift.prevent="go(-1)"
      />
      <span v-if="query" class="count" aria-live="polite">
        {{ hits.length ? `${current + 1} / ${hits.length}` : t('compile.log.noMatches') }}
      </span>
      <button :disabled="!hits.length" :aria-label="t('compile.log.previous')" @click="go(-1)">
        ↑
      </button>
      <button :disabled="!hits.length" :aria-label="t('compile.log.next')" @click="go(1)">↓</button>
      <select v-model="source" :aria-label="t('compile.log.source')">
        <option value="log">{{ t('compile.log.texLog') }}</option>
        <option value="output">{{ t('compile.log.latexmk') }}</option>
      </select>
    </div>
    <p v-if="!text" class="muted">{{ t('compile.log.empty') }}</p>
    <pre v-else ref="body" class="body"><code><template v-for="(line, i) in lines" :key="i"><span
      v-for="(s, j) in segments(i, line)"
      :key="j"
    ><mark v-if="s.hit >= 0" :class="{ current: s.hit === current }">{{ s.text }}</mark><template
      v-else>{{ s.text }}</template></span>
</template></code></pre>
  </div>
</template>

<style scoped>
.log {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
.bar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-2);
}
.bar input {
  flex: 1;
  min-width: 0;
  background: var(--bg-elevated);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 1px var(--space-2);
}
.bar button,
.bar select {
  background: var(--bg-elevated);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.count,
.muted {
  color: var(--text-muted);
  margin: 0;
}
.body {
  flex: 1;
  min-height: 0;
  margin: 0;
  overflow: auto;
  font-family: var(--font-mono, monospace);
  font-size: 12px;
  white-space: pre;
}
mark {
  background: color-mix(in srgb, var(--accent) 35%, transparent);
  color: inherit;
}
mark.current {
  background: var(--accent);
  color: var(--accent-contrast);
}
</style>
