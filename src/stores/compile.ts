import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import {
  compileCancel,
  compileStart,
  getManifest,
  saveManifest,
  synctexForward,
  synctexInverse,
  toCompileError,
  type CompileError,
  type CompileOutcome,
  type ForwardResult,
  type Manifest,
} from '../compile/api'
import { countDiagnostics, isCompileRelevant, sortDiagnostics } from '../compile/model'
import { t } from '../i18n'
import { joinPath } from '../project/api'
import { useEditorStore } from './editor'
import { useNotificationsStore } from './notifications'
import { useProjectStore } from './project'
import { useSettingsStore } from './settings'
import { useUiStore } from './ui'

export type CompileState =
  'idle' | 'running' | 'success' | 'failed' | 'cancelled' | 'timedOut' | 'error'

/** Alvo de destaque no PDF depois de uma busca direta do SyncTeX. */
export interface SyncTarget extends ForwardResult {
  nonce: number
}

const DEFAULT_DEBOUNCE_MS = 1500

/** Pergunta de segurança antes de ligar `shell-escape` (permite executar comandos do documento). */
async function confirmShellEscape(): Promise<boolean> {
  const message = t('compile.shellEscape.warning')
  const title = t('compile.shellEscape.title')
  try {
    const { ask } = await import('@tauri-apps/plugin-dialog')
    return await ask(message, { title, kind: 'warning' })
  } catch {
    return typeof window !== 'undefined' && typeof window.confirm === 'function'
      ? window.confirm(message)
      : false
  }
}

export const useCompileStore = defineStore('compile', () => {
  const editor = useEditorStore()
  const project = useProjectStore()
  const settings = useSettingsStore()
  const notifications = useNotificationsStore()

  const state = ref<CompileState>('idle')
  const outcome = ref<CompileOutcome | null>(null)
  const error = ref<CompileError | null>(null)
  const manifest = ref<Manifest | null>(null)
  /** Incrementa quando um PDF novo foi gerado: o visualizador recarrega. */
  const pdfVersion = ref(0)
  /** Há uma mudança aguardando o fim da compilação em andamento. */
  const queued = ref(false)
  const showBoxes = ref(false)
  const syncTarget = ref<SyncTarget | null>(null)
  let syncNonce = 0

  const diagnostics = computed(() => sortDiagnostics(outcome.value?.diagnostics ?? []))
  const counts = computed(() => countDiagnostics(diagnostics.value, showBoxes.value))
  const auto = computed(() => manifest.value?.compile.auto ?? true)
  const engine = computed(() => manifest.value?.tex.engine ?? null)
  const shellEscape = computed(() => manifest.value?.compile.shell_escape ?? false)
  const debounceMs = computed(() => manifest.value?.compile.debounce_ms ?? DEFAULT_DEBOUNCE_MS)

  /** Caminho esperado do PDF: `<outdir>/<principal>.pdf`. */
  const pdfPath = computed(() => {
    const m = manifest.value
    if (!project.root || !m) return null
    const stem = (m.project.main.split('/').pop() ?? 'main').replace(/\.tex$/i, '')
    return joinPath(project.root, `${m.compile.outdir}/${stem}.pdf`)
  })

  // ───────────────────────── manifesto ─────────────────────────

  async function loadManifest() {
    manifest.value = null
    if (!project.root) return
    try {
      manifest.value = await getManifest(project.root)
    } catch (e) {
      notifications.notify('error', String(e))
    }
  }

  async function updateManifest(patch: (m: Manifest) => void): Promise<boolean> {
    if (!project.root || !manifest.value) return false
    const next = JSON.parse(JSON.stringify(manifest.value)) as Manifest
    patch(next)
    try {
      await saveManifest(project.root, next)
      manifest.value = next
      return true
    } catch (e) {
      notifications.notify('error', String(e))
      return false
    }
  }

  async function setAuto(on: boolean) {
    const ok = await updateManifest((m) => (m.compile.auto = on))
    if (ok && !on) clearTimer()
  }

  async function toggleAuto() {
    await setAuto(!auto.value)
  }

  /** Ligar exige confirmação (aviso de segurança); desligar é imediato. */
  async function setShellEscape(on: boolean): Promise<boolean> {
    if (on && !(await confirmShellEscape())) return false
    return updateManifest((m) => (m.compile.shell_escape = on))
  }

  // ───────────────────────── compilação ─────────────────────────

  function relevantDirty(): string[] {
    return editor.tabs.filter((x) => x.dirty && isCompileRelevant(x.path)).map((x) => x.path)
  }

  let savingForCompile = false
  async function saveRelevant() {
    savingForCompile = true
    try {
      await Promise.all(relevantDirty().map((p) => editor.save(p)))
    } finally {
      savingForCompile = false
    }
  }

  /**
   * Compila agora. Se já houver uma compilação do projeto, enfileira uma nova para quando ela
   * terminar (nunca duas em paralelo). `manual` mostra avisos de erro ao usuário.
   */
  async function compile(opts: { manual?: boolean; save?: boolean } = {}): Promise<void> {
    const root = project.root
    if (!root) return
    clearTimer()
    if (state.value === 'running') {
      queued.value = true
      return
    }
    state.value = 'running'
    error.value = null
    try {
      if (opts.save ?? true) await saveRelevant()
      const result = await compileStart(root, settings.effective.texBinDir)
      outcome.value = result
      state.value = result.status
      if (result.pdfUpdated) pdfVersion.value++
      if (result.status === 'timedOut' && opts.manual) {
        notifications.notify('error', t('compile.timedOut'))
      }
    } catch (e) {
      const err = toCompileError(e)
      error.value = err
      // "busy": o backend ainda tinha uma compilação deste projeto; não é uma falha.
      state.value = err.kind === 'busy' ? 'idle' : 'error'
      if (opts.manual && err.kind !== 'busy') {
        notifications.notify('error', `${t(err.i18nKey)}${err.details ? ` (${err.details})` : ''}`)
      }
    } finally {
      if (queued.value) {
        queued.value = false
        void compile({ manual: false })
      }
    }
  }

  async function cancel() {
    queued.value = false
    if (project.root && state.value === 'running') await compileCancel(project.root)
  }

  // ───────────────────────── compilação automática (TL-402) ─────────────────────────

  let timer: ReturnType<typeof setTimeout> | undefined
  function clearTimer() {
    clearTimeout(timer)
    timer = undefined
  }

  function schedule() {
    clearTimer()
    timer = setTimeout(() => {
      timer = undefined
      void autoRun()
    }, debounceMs.value)
  }

  async function autoRun() {
    if (!auto.value || !project.root) return
    // Com o autosave desligado o usuário controla quando gravar: compila só o que está em disco.
    if (editor.prefs.autosave !== 'off') await saveRelevant()
    if (relevantDirty().length) return
    await compile({ manual: false, save: false })
  }

  watch(
    () => editor.change,
    (c) => {
      if (auto.value && isCompileRelevant(c.path)) schedule()
    },
  )

  watch(
    () => editor.saved,
    (s) => {
      if (/(^|[\\/])texlocal\.toml$/i.test(s.path)) void loadManifest()
      if (savingForCompile || !auto.value || !isCompileRelevant(s.path)) return
      // O autosave grava no meio da digitação; a pausa é contada a partir da última edição.
      if (timer === undefined) schedule()
    },
  )

  watch(
    () => project.root,
    () => {
      clearTimer()
      outcome.value = null
      state.value = 'idle'
      error.value = null
      queued.value = false
      syncTarget.value = null
      void loadManifest()
    },
    { immediate: true },
  )

  /** Ctrl+S: com a compilação automática ligada, compila já em vez de esperar a pausa. */
  async function afterExplicitSave(path: string) {
    if (auto.value && isCompileRelevant(path)) await compile({ manual: false, save: false })
  }

  // ───────────────────────── SyncTeX (TL-406) ─────────────────────────

  /** Leva o PDF à posição do cursor. */
  async function syncToPdf(): Promise<boolean> {
    const file = editor.active
    if (!project.root || !file) return false
    try {
      const result = await synctexForward(project.root, file, editor.cursor.line)
      if (!result || !result.rects.length) {
        notifications.notify('info', t('compile.sync.noPosition'))
        return false
      }
      syncTarget.value = { ...result, nonce: ++syncNonce }
      const ui = useUiStore()
      if (ui.layout.viewMode === 'editor') ui.setViewMode('split')
      return true
    } catch {
      notifications.notify('info', t('compile.sync.unavailable'))
      return false
    }
  }

  /** Abre no editor o trecho correspondente ao ponto clicado no PDF (bp, canto superior esquerdo). */
  async function syncToSource(page: number, x: number, y: number): Promise<boolean> {
    if (!project.root) return false
    try {
      const pos = await synctexInverse(project.root, page, x, y)
      if (!pos) {
        notifications.notify('info', t('compile.sync.noPosition'))
        return false
      }
      await editor.openFile(pos.file, { line: Math.max(pos.line - 1, 0), col: 0 })
      return true
    } catch {
      notifications.notify('info', t('compile.sync.unavailable'))
      return false
    }
  }

  function reset() {
    clearTimer()
    outcome.value = null
    state.value = 'idle'
  }

  return {
    state,
    outcome,
    error,
    manifest,
    pdfVersion,
    pdfPath,
    queued,
    showBoxes,
    syncTarget,
    diagnostics,
    counts,
    auto,
    engine,
    shellEscape,
    loadManifest,
    setAuto,
    toggleAuto,
    setShellEscape,
    compile,
    cancel,
    afterExplicitSave,
    syncToPdf,
    syncToSource,
    reset,
  }
})
