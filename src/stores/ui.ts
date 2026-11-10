import { defineStore } from 'pinia'
import { reactive, ref, watch } from 'vue'

export type ViewMode = 'split' | 'editor' | 'pdf'
export type PaletteMode = 'commands' | 'files' | null
export type SidebarPanel = 'files' | 'search'

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
  const wizardOpen = ref(false)
  const sidebarPanel = ref<SidebarPanel>('files')
  /** Incrementa para pedir foco no campo de busca do projeto. */
  const searchFocusTick = ref(0)

  function save(id: string) {
    try {
      localStorage.setItem(storageKey(id), JSON.stringify(layout))
    } catch {
      // ignora falhas de persistência
    }
  }

  let persistTimer: ReturnType<typeof setTimeout> | undefined
  watch(
    layout,
    () => {
      // Debounce: arrastar divisórias muda o layout dezenas de vezes por segundo.
      clearTimeout(persistTimer)
      persistTimer = setTimeout(() => save(project.value), 200)
    },
    { deep: true },
  )
  // Não perde o último ajuste se a janela fechar dentro da janela do debounce.
  if (typeof window !== 'undefined') {
    window.addEventListener('pagehide', () => save(project.value))
  }

  function setProject(id: string) {
    // Grava o layout do projeto atual antes de trocar, sem esperar o debounce.
    clearTimeout(persistTimer)
    save(project.value)
    project.value = id
    Object.assign(layout, load(id))
  }

  function toggleSidebar() {
    layout.sidebarVisible = !layout.sidebarVisible
  }
  function toggleBottom() {
    layout.bottomVisible = !layout.bottomVisible
  }
  function showSidebarPanel(panel: SidebarPanel) {
    if (layout.sidebarVisible && sidebarPanel.value === panel && panel === 'files') {
      layout.sidebarVisible = false
      return
    }
    sidebarPanel.value = panel
    layout.sidebarVisible = true
    if (panel === 'search') searchFocusTick.value++
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
    wizardOpen,
    sidebarPanel,
    searchFocusTick,
    showSidebarPanel,
    setProject,
    toggleSidebar,
    toggleBottom,
    setViewMode,
    resize,
  }
})
