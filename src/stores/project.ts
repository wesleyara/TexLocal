import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
  emptyView,
  indexRemove,
  indexUpdate,
  joinPath,
  pickFolder,
  projectFiles,
  projectOpen,
  relativePath,
  spellConfigure,
  type ProjectView,
} from '../project/api'
import {
  allLabels,
  buildOutline,
  documentClass,
  loadedPackages,
  pathKey,
  userCommands,
} from '../project/model'

const LAST_KEY = 'texlocal.project.last.v1'

export const useProjectStore = defineStore('project', () => {
  const root = ref<string | null>(null)
  /** Arquivos do projeto, relativos à raiz (com `/`). */
  const files = ref<string[]>([])
  const view = ref<ProjectView>(emptyView())
  /** Idioma definido no manifesto (`[editor] language`). */
  const manifestLanguage = ref<string | null>(null)
  /** Idioma detectado pelo babel no documento principal. */
  const detectedLanguage = ref<string | null>(null)
  const dictionaries = ref<string[]>([])

  const outline = computed(() => buildOutline(view.value))
  const labels = computed(() => allLabels(view.value))
  const packages = computed(() => (view.value.main ? loadedPackages(view.value) : null))
  const docClass = computed(() => documentClass(view.value))
  const commands = computed(() => userCommands(view.value))
  const mainDir = computed(() => {
    const main = view.value.main
    if (!main) return root.value
    const p = pathKey(main)
    return p.slice(0, p.lastIndexOf('/'))
  })
  /** Arquivos relativos à pasta do principal, para completar `\input` e `\includegraphics`. */
  const filesFromMain = computed(() => {
    if (!root.value || !mainDir.value) return files.value
    return files.value.map((f) =>
      relativePath(mainDir.value as string, joinPath(root.value as string, f)),
    )
  })
  const spellLanguage = computed(() => manifestLanguage.value ?? detectedLanguage.value)

  async function open(path: string) {
    const opened = await projectOpen(path)
    root.value = opened.root
    view.value = opened.view
    manifestLanguage.value = opened.language
    files.value = await projectFiles(opened.root)
    dictionaries.value = await spellConfigure(opened.root)
    try {
      localStorage.setItem(LAST_KEY, opened.root)
    } catch {
      // ignora
    }
  }

  async function openFolder(): Promise<boolean> {
    const picked = await pickFolder()
    if (!picked) return false
    await open(picked)
    return true
  }

  function lastRoot(): string | null {
    try {
      return localStorage.getItem(LAST_KEY)
    } catch {
      return null
    }
  }

  async function refreshFiles() {
    if (root.value) files.value = await projectFiles(root.value)
  }

  /** Atualização incremental do índice com o texto do buffer. */
  async function updateIndex(path: string, text: string, hint = false) {
    if (!root.value) return
    view.value = await indexUpdate(root.value, path, text, hint)
  }

  async function removeFromIndex(path: string) {
    if (!root.value) return
    view.value = await indexRemove(root.value, path)
  }

  function absolute(rel: string): string {
    return root.value ? joinPath(root.value, rel) : rel
  }

  return {
    root,
    files,
    view,
    manifestLanguage,
    detectedLanguage,
    dictionaries,
    outline,
    labels,
    packages,
    docClass,
    commands,
    mainDir,
    filesFromMain,
    spellLanguage,
    open,
    openFolder,
    lastRoot,
    refreshFiles,
    updateIndex,
    removeFromIndex,
    absolute,
  }
})
