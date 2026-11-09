import { defineStore } from 'pinia'
import { reactive, ref, watch } from 'vue'

export type ViewMode = 'split' | 'editor' | 'pdf'
export type PaletteMode = 'commands' | 'files' | null

export interface LayoutState {
  sidebarVisible: boolean
  bottomVisible: boolean
  sidebarWidth: number
  pdfWidth: number
  bottomHeight: number
  viewMode: ViewMode
}

export const LIMITS = {
  sidebarWidth: { min: 160, max: 480 },
  pdfWidth: { min: 240, max: 1200 },
  bottomHeight: { min: 80, max: 480 },
}

export const DEFAULT_LAYOUT: LayoutState = {
  sidebarVisible: true,
  bottomVisible: false,
  sidebarWidth: 240,
  pdfWidth: 480,
  bottomHeight: 180,
  viewMode: 'split',
}

export const clamp = (v: number, min: number, max: number): number =>
  Math.min(max, Math.max(min, v))

/** Layout é persistido por projeto; enquanto não há projetos (EP08), usa a chave "default". */
const storageKey = (project: string) => `texlocal.layout.v1.${project}`

function load(project: string): LayoutState {
  try {
    const raw = localStorage.getItem(storageKey(project))
    if (raw) return { ...DEFAULT_LAYOUT, ...(JSON.parse(raw) as Partial<LayoutState>) }
  } catch {
    // armazenamento indisponível ou corrompido: usa o padrão
  }
  return { ...DEFAULT_LAYOUT }
}

export const useUiStore = defineStore('ui', () => {
  const project = ref('default')
  const layout = reactive<LayoutState>(load(project.value))
  const paletteMode = ref<PaletteMode>(null)
  const settingsOpen = ref(false)

  watch(
    layout,
    () => {
      try {
        localStorage.setItem(storageKey(project.value), JSON.stringify(layout))
      } catch {
        // ignora falhas de persistência
      }
    },
    { deep: true },
  )

  function setProject(id: string) {
    project.value = id
    Object.assign(layout, load(id))
  }

  function toggleSidebar() {
    layout.sidebarVisible = !layout.sidebarVisible
  }
  function toggleBottom() {
    layout.bottomVisible = !layout.bottomVisible
  }
  function setViewMode(mode: ViewMode) {
    layout.viewMode = mode
  }
  function resize(key: 'sidebarWidth' | 'pdfWidth' | 'bottomHeight', value: number) {
    layout[key] = clamp(value, LIMITS[key].min, LIMITS[key].max)
  }

  return {
    project,
    layout,
    paletteMode,
    settingsOpen,
    setProject,
    toggleSidebar,
    toggleBottom,
    setViewMode,
    resize,
  }
})
