import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { buffers } from '../editor/buffers'
import { languageIdFor } from '../editor/languages'
import { baseName, fileRead, fileWrite, type Encoding, type Eol } from '../project/api'
import { pathKey, samePath } from '../project/model'
import { i18n } from '../i18n'
import { useNotificationsStore } from './notifications'
import { useProjectStore } from './project'

export interface Tab {
  path: string
  name: string
  dirty: boolean
  eol: Eol
  encoding: Encoding
  bom: boolean
  /** O fim de linha foi alterado e ainda não gravado. */
  eolChanged: boolean
}

export type AutosaveMode = 'off' | 'delay' | 'blur' | 'both'

export interface EditorPrefs {
  autosave: AutosaveMode
  autosaveDelayMs: number
  wrap: boolean
}

export const DEFAULT_PREFS: EditorPrefs = { autosave: 'both', autosaveDelayMs: 1000, wrap: true }

const PREFS_KEY = 'texlocal.editor.prefs.v1'
const sessionKey = (root: string) => `texlocal.editor.session.v1.${root}`

function loadPrefs(): EditorPrefs {
  try {
    const raw = localStorage.getItem(PREFS_KEY)
    if (raw) return { ...DEFAULT_PREFS, ...(JSON.parse(raw) as Partial<EditorPrefs>) }
  } catch {
    // padrão
  }
  return { ...DEFAULT_PREFS }
}

/** Posição a revelar depois de abrir/ativar um arquivo (0-based). */
export interface Reveal {
  path: string
  line: number
  col: number
  endCol?: number
  nonce: number
}

export const useEditorStore = defineStore('editor', () => {
  const tabs = ref<Tab[]>([])
  const active = ref<string | null>(null)
  const prefs = ref<EditorPrefs>(loadPrefs())
  const reveal = ref<Reveal | null>(null)
  const reloadTick = ref(0)
  const cursor = ref({ line: 1, col: 1, selectionWords: null as number | null })
  let nonce = 0
  const timers = new Map<string, ReturnType<typeof setTimeout>>()

  const activeTab = computed(() =>
    tabs.value.find((t) => active.value && samePath(t.path, active.value)),
  )

  const t = (key: string, params?: Record<string, unknown>) =>
    i18n.global.t(key, params ?? {}) as string

  function find(path: string): Tab | undefined {
    return tabs.value.find((x) => samePath(x.path, path))
  }

  function persistSession() {
    const root = useProjectStore().root
    if (!root) return
    try {
      localStorage.setItem(
        sessionKey(root),
        JSON.stringify({ paths: tabs.value.map((x) => x.path), active: active.value }),
      )
    } catch {
      // ignora
    }
  }

  function setPrefs(patch: Partial<EditorPrefs>) {
    prefs.value = { ...prefs.value, ...patch }
    try {
      localStorage.setItem(PREFS_KEY, JSON.stringify(prefs.value))
    } catch {
      // ignora
    }
  }

  async function openFile(path: string, at?: { line: number; col?: number; endCol?: number }) {
    let tab = find(path)
    if (!tab) {
      const content = await fileRead(path)
      buffers.setPending(path, content.text)
      tab = {
        path,
        name: baseName(path),
        dirty: false,
        eol: content.eol,
        encoding: content.encoding,
        bom: content.bom,
        eolChanged: false,
      }
      tabs.value.push(tab)
      if (content.encoding === 'latin1') {
        useNotificationsStore().notify('info', t('editor.latin1Warning', { name: tab.name }), 9000)
      }
    }
    active.value = tab.path
    if (at)
      reveal.value = {
        path: tab.path,
        line: at.line,
        col: at.col ?? 0,
        endCol: at.endCol,
        nonce: ++nonce,
      }
    persistSession()
  }

  function activate(path: string) {
    if (find(path)) {
      active.value = find(path)!.path
      persistSession()
    }
  }

  async function save(path: string): Promise<boolean> {
    const tab = find(path)
    if (!tab) return false
    const text = buffers.text(tab.path)
    if (text === undefined) return false
    clearTimeout(timers.get(pathKey(path)))
    try {
      const result = await fileWrite(tab.path, text, tab.eol, tab.encoding, tab.bom)
      if (result.encoding !== tab.encoding) {
        tab.encoding = result.encoding
        useNotificationsStore().notify('info', t('editor.latin1Converted', { name: tab.name }))
      }
      buffers.markSaved(tab.path)
      tab.eolChanged = false
      tab.dirty = buffers.isDirty(tab.path)
      return true
    } catch (e) {
      useNotificationsStore().notify(
        'error',
        t('editor.saveFailed', { name: tab.name, error: String(e) }),
      )
      return false
    }
  }

  async function saveAll() {
    await Promise.all(tabs.value.filter((x) => x.dirty).map((x) => save(x.path)))
  }

  /** Chamado pelo editor a cada mudança do documento. */
  function onDocChanged(path: string) {
    const tab = find(path)
    if (!tab) return
    tab.dirty = buffers.isDirty(tab.path) || tab.eolChanged
    const k = pathKey(path)
    clearTimeout(timers.get(k))
    if (tab.dirty && (prefs.value.autosave === 'delay' || prefs.value.autosave === 'both')) {
      timers.set(
        k,
        setTimeout(() => void save(path), prefs.value.autosaveDelayMs),
      )
    }
  }

  /** Perda de foco do editor ou da janela. */
  function onBlur() {
    if (prefs.value.autosave === 'blur' || prefs.value.autosave === 'both') void saveAll()
  }

  async function close(path: string, opts: { discard?: boolean } = {}): Promise<boolean> {
    const tab = find(path)
    if (!tab) return true
    if (tab.dirty && !opts.discard) {
      const ok = await save(tab.path)
      if (!ok) return false
    }
    clearTimeout(timers.get(pathKey(path)))
    const i = tabs.value.indexOf(tab)
    tabs.value.splice(i, 1)
    buffers.drop(tab.path)
    if (active.value && samePath(active.value, tab.path)) {
      active.value = (tabs.value[i] ?? tabs.value[i - 1])?.path ?? null
    }
    persistSession()
    return true
  }

  async function closeOthers(path: string) {
    for (const tab of [...tabs.value]) if (!samePath(tab.path, path)) await close(tab.path)
  }

  async function closeToRight(path: string) {
    const i = tabs.value.findIndex((x) => samePath(x.path, path))
    for (const tab of tabs.value.slice(i + 1)) await close(tab.path)
  }

  function move(from: number, to: number) {
    if (from === to || from < 0 || to < 0 || from >= tabs.value.length || to >= tabs.value.length)
      return
    const [tab] = tabs.value.splice(from, 1)
    tabs.value.splice(to, 0, tab)
    persistSession()
  }

  function setEol(path: string, eol: Eol) {
    const tab = find(path)
    if (!tab || tab.eol === eol) return
    // O novo fim de linha vale ao salvar; até lá o arquivo conta como alterado.
    tab.eol = eol
    tab.eolChanged = true
    onDocChanged(path)
  }

  /** Reabre as abas do projeto atual (ignora arquivos que sumiram). */
  async function restoreSession() {
    const root = useProjectStore().root
    if (!root) return
    try {
      const raw = localStorage.getItem(sessionKey(root))
      if (!raw) return
      const s = JSON.parse(raw) as { paths: string[]; active: string | null }
      for (const p of s.paths) {
        try {
          await openFile(p)
        } catch {
          // arquivo removido: pula
        }
      }
      if (s.active && find(s.active)) active.value = find(s.active)!.path
    } catch {
      // sessão corrompida: ignora
    }
  }

  /** Fecha tudo (troca de projeto). */
  function reset() {
    tabs.value = []
    active.value = null
    buffers.reset()
  }

  /** Recarrega do disco um arquivo aberto sem alterações (após substituição em lote). */
  async function reloadFromDisk(path: string) {
    const tab = find(path)
    if (!tab || tab.dirty) return
    const content = await fileRead(tab.path)
    buffers.drop(tab.path)
    buffers.setPending(tab.path, content.text)
    tab.eol = content.eol
    tab.encoding = content.encoding
    reloadTick.value++
  }

  const languageId = computed(() => (active.value ? languageIdFor(active.value) : 'text'))

  return {
    tabs,
    active,
    activeTab,
    prefs,
    reveal,
    reloadTick,
    cursor,
    languageId,
    setPrefs,
    openFile,
    activate,
    save,
    saveAll,
    onDocChanged,
    onBlur,
    close,
    closeOthers,
    closeToRight,
    move,
    setEol,
    restoreSession,
    reset,
    reloadFromDisk,
  }
})
