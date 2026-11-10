import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useUiStore } from './ui'

export interface Location {
  file: string
  line: number
  col: number
  endCol: number
}

export const useRefsStore = defineStore('refs', () => {
  /** Label cujo uso está sendo listado no painel inferior. */
  const name = ref<string | null>(null)
  const definition = ref<Location | null>(null)
  const usages = ref<Location[]>([])
  /** Label em renomeação (abre o diálogo). */
  const renaming = ref<string | null>(null)

  function showUsages(label: string, def: Location | null, uses: Location[]) {
    name.value = label
    definition.value = def
    usages.value = uses
    useUiStore().layout.bottomVisible = true
  }

  function clear() {
    name.value = null
    definition.value = null
    usages.value = []
  }

  return { name, definition, usages, renaming, showUsages, clear }
})
