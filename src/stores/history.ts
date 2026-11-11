import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { t } from '../i18n'
import {
  emptyStatus,
  historyCheckpoint,
  historyCommit,
  historyExportPdf,
  historyExportZip,
  historyInit,
  historyLog,
  historyRestore,
  historyRestoreText,
  historyStage,
  historyStatus,
  historyTagCreate,
  historyTagDelete,
  historyTagRename,
  identityGet,
  identitySet,
  isBusyError,
  isIdentityError,
  pickSavePath,
  type CommitInfo,
  type Identity,
  type LogQuery,
  type RepoStatus,
  type RestoreResult,
  type TagInfo,
} from '../history/api'
import { isTauri } from '../tauri'
import { useEditorStore } from './editor'
import { useNotificationsStore } from './notifications'
import { useProjectStore } from './project'
import { useSettingsStore } from './settings'

const PAGE = 50

export interface HistoryConfig {
  /** Checkpoints automáticos ligados. */
  auto: boolean
  /** Minutos de inatividade (com mudanças) antes do checkpoint. */
  minutes: number
}
export const DEFAULT_CONFIG: HistoryConfig = { auto: true, minutes: 5 }

const key = (kind: string, root: string) => `texlocal.history.${kind}.v1.${root}`

function readJson<T>(k: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(k)
    return raw ? { ...fallback, ...(JSON.parse(raw) as object) } : fallback
  } catch {
    return fallback
  }
}
function writeJson(k: string, v: unknown) {
  try {
    localStorage.setItem(k, JSON.stringify(v))
  } catch {
    // ignora
  }
}

/** Ponta de uma comparação: um commit, ou `workdir` (estado atual dos arquivos). */
export type CompareEnd = CommitInfo | 'workdir'
export interface Comparison {
  /** `null` = projeto vazio (antes do primeiro commit). */
  from: CommitInfo | null
  to: CompareEnd
}

export const useHistoryStore = defineStore('history', () => {
  const project = useProjectStore()
  const editor = useEditorStore()
  const settings = useSettingsStore()
  const notify = useNotificationsStore()

  const status = ref<RepoStatus>(emptyStatus())
  const commits = ref<CommitInfo[]>([])
  const hasMore = ref(false)
  const loading = ref(false)
  const busy = ref(false)
  const filters = ref({ namedOnly: false, path: '', author: '' })
  const config = ref<HistoryConfig>({ ...DEFAULT_CONFIG })
  const identity = ref<Identity | null>(null)
  const identityDialog = ref(false)
  const declined = ref(false)
  const comparison = ref<Comparison | null>(null)
  const lastCheckpointAt = ref<number | null>(null)
  /** Diálogo de nomear/renomear versão (`target` null = versão atual). */
  const nameDialog = ref<{ target: string | null; tag?: TagInfo } | null>(null)

  const initialized = computed(() => status.value.initialized)
  const dirty = computed(() => status.value.entries.length > 0)
  const merging = computed(() => status.value.state !== 'clean')
  const root = computed(() => project.root)

  const identityWaiters: ((id: Identity | null) => void)[] = []

  // ───────── identidade ─────────

  async function ensureIdentity(): Promise<Identity | null> {
    if (identity.value) return identity.value
    identity.value = await identityGet().catch(() => null)
    if (identity.value) return identity.value
    identityDialog.value = true
    return new Promise((resolve) => identityWaiters.push(resolve))
  }

  async function saveIdentity(name: string, email: string): Promise<string | null> {
    try {
      identity.value = await identitySet(name, email)
    } catch (e) {
      return String(e)
    }
    identityDialog.value = false
    identityWaiters.splice(0).forEach((r) => r(identity.value))
    return null
  }

  function cancelIdentity() {
    identityDialog.value = false
    identityWaiters.splice(0).forEach((r) => r(null))
  }

  /** Executa `fn`; se faltar identidade, pergunta e tenta de novo uma vez. */
  async function withIdentity<T>(fn: () => Promise<T>): Promise<T | undefined> {
    if (!(await ensureIdentity())) return undefined
    try {
      return await fn()
    } catch (e) {
      if (!isIdentityError(e)) throw e
      identity.value = null
      if (!(await ensureIdentity())) return undefined
      return fn()
    }
  }

  // ───────── carga ─────────

  function query(skip: number): LogQuery {
    return {
      skip,
      limit: PAGE,
      namedOnly: filters.value.namedOnly,
      path: filters.value.path.trim() || null,
      author: filters.value.author.trim() || null,
    }
  }

  async function refreshStatus() {
    if (!root.value) {
      status.value = emptyStatus()
      return
    }
    status.value = await historyStatus(root.value).catch(() => emptyStatus())
  }

  async function reloadLog() {
    if (!root.value || !status.value.initialized) {
      commits.value = []
      hasMore.value = false
      return
    }
    loading.value = true
    try {
      const page = await historyLog(root.value, query(0))
      commits.value = page
      hasMore.value = page.length === PAGE
    } catch (e) {
      notify.notify('error', String(e))
    } finally {
      loading.value = false
    }
  }

  async function loadMore() {
    if (!root.value || loading.value || !hasMore.value) return
    loading.value = true
    try {
      const page = await historyLog(root.value, query(commits.value.length))
      commits.value.push(...page)
      hasMore.value = page.length === PAGE
    } finally {
      loading.value = false
    }
  }

  async function refresh() {
    await refreshStatus()
    await reloadLog()
  }

  let filterTimer: ReturnType<typeof setTimeout> | undefined
  watch(
    filters,
    () => {
      clearTimeout(filterTimer)
      filterTimer = setTimeout(() => void reloadLog(), 250)
    },
    { deep: true },
  )

  // ───────── operações ─────────

  async function run<T>(fn: () => Promise<T>, quiet = false): Promise<T | undefined> {
    busy.value = true
    try {
      return await withIdentity(fn)
    } catch (e) {
      if (!quiet) notify.notify('error', isBusyError(e) ? t('history.busy') : String(e))
      return undefined
    } finally {
      busy.value = false
    }
  }

  async function enable() {
    const r = root.value
    if (!r) return
    const c = await run(() => historyInit(r))
    declined.value = false
    writeJson(key('declined', r), { declined: false })
    await refresh()
    if (c !== undefined) notify.notify('success', t('history.enabled'))
  }

  function decline() {
    if (!root.value) return
    declined.value = true
    writeJson(key('declined', root.value), { declined: true })
  }

  /** Salva os arquivos abertos e cria um checkpoint (se houver mudanças). */
  async function checkpoint(
    opts: { message?: string; quiet?: boolean } = {},
  ): Promise<CommitInfo | null> {
    const r = root.value
    if (!r || !status.value.initialized) return null
    await editor.saveAll()
    const c = await run(() => historyCheckpoint(r, opts.message), opts.quiet)
    await refresh()
    if (c) {
      lastCheckpointAt.value = Date.now()
      if (!opts.quiet) notify.notify('success', t('history.checkpointDone', { n: c.filesChanged }))
    } else if (c === null && !opts.quiet) {
      notify.notify('info', t('history.nothingToSave'))
    }
    return c ?? null
  }

  async function commitManual(message: string, amend: boolean): Promise<boolean> {
    const r = root.value
    if (!r) return false
    await editor.saveAll()
    const c = await run(() => historyCommit(r, message, amend))
    await refresh()
    if (c) {
      notify.notify('success', t('history.committed'))
      return true
    }
    if (c === null) notify.notify('info', t('history.nothingStaged'))
    return false
  }

  async function stage(paths: string[], staged: boolean) {
    if (!root.value) return
    try {
      await historyStage(root.value, paths, staged)
    } catch (e) {
      notify.notify('error', String(e))
    }
    await refreshStatus()
  }

  async function nameVersion(label: string, description: string, target: string | null) {
    const r = root.value
    if (!r) return
    if (target === null) await editor.saveAll()
    const tag = await run(() => historyTagCreate(r, target, label, description))
    await refresh()
    if (tag) notify.notify('success', t('history.named', { name: tag.label }))
  }

  async function renameVersion(name: string, label: string, description: string | null) {
    const r = root.value
    if (!r) return
    await run(() => historyTagRename(r, name, label, description))
    await reloadLog()
  }

  async function removeVersion(name: string) {
    const r = root.value
    if (!r) return
    await run(() => historyTagDelete(r, name))
    await reloadLog()
  }

  /** Recarrega do disco os arquivos abertos que a restauração alterou. */
  async function afterRestore(res: RestoreResult) {
    for (const rel of res.touched) {
      const abs = project.absolute(rel)
      const tab = editor.tabs.find((x) => x.path.replace(/\\/g, '/') === abs.replace(/\\/g, '/'))
      if (!tab) continue
      tab.dirty = false
      await editor.reloadFromDisk(tab.path).catch(() => undefined)
    }
    await project.refreshFiles()
    await refresh()
  }

  async function restore(
    target: string,
    paths: string[] | null,
  ): Promise<RestoreResult | undefined> {
    const r = root.value
    if (!r) return undefined
    await editor.saveAll()
    const res = await run(() => historyRestore(r, target, paths))
    if (res) {
      await afterRestore(res)
      notify.notify('success', res.commit ? t('history.restored') : t('history.restoredNoop'))
    }
    return res
  }

  async function restoreText(source: string, path: string, text: string) {
    const r = root.value
    if (!r) return
    await editor.saveAll()
    const res = await run(() => historyRestoreText(r, source, path, text))
    if (res) {
      await afterRestore(res)
      notify.notify('success', t('history.restoredHunk'))
    }
  }

  async function exportZip(c: CommitInfo) {
    const r = root.value
    if (!r) return
    const dest = await pickSavePath(`${slug(c)}.zip`, 'ZIP', 'zip')
    if (!dest) return
    try {
      const n = await historyExportZip(r, c.id, dest)
      notify.notify('success', t('history.exported', { n }))
    } catch (e) {
      notify.notify('error', String(e))
    }
  }

  async function exportPdf(c: CommitInfo) {
    const r = root.value
    if (!r) return
    const dest = await pickSavePath(`${slug(c)}.pdf`, 'PDF', 'pdf')
    if (!dest) return
    const task = notify.startTask(t('history.exportingPdf', { name: slug(c) }))
    try {
      await historyExportPdf(r, c.id, dest, settings.effective.texBinDir ?? null)
      notify.notify('success', t('history.exportedPdf'))
    } catch (e) {
      notify.notify('error', String(e))
    } finally {
      notify.finishTask(task)
    }
  }

  const slug = (c: CommitInfo) =>
    (c.tags[0]?.label ?? c.short).replace(/[^\p{L}\p{N}_-]+/gu, '-').replace(/^-+|-+$/g, '') ||
    c.short

  // ───────── comparação ─────────

  function compare(from: CommitInfo | null, to: CompareEnd) {
    comparison.value = { from, to }
  }
  function compareWithCurrent(c: CommitInfo) {
    compare(c, 'workdir')
  }
  /** Mudanças introduzidas por um commit (pai → commit). */
  function compareCommit(c: CommitInfo) {
    const parent = commits.value.find((x) => x.id === c.parent) ?? null
    comparison.value = { from: parent ?? (c.parent ? stub(c.parent) : null), to: c }
  }
  /** Alterações ainda não salvas no histórico (último commit → arquivos atuais). */
  function compareWorkdir() {
    comparison.value = { from: stub('HEAD'), to: 'workdir' }
  }
  function closeComparison() {
    comparison.value = null
  }

  // ───────── checkpoints automáticos ─────────

  function setConfig(patch: Partial<HistoryConfig>) {
    config.value = { ...config.value, ...patch }
    if (root.value) writeJson(key('cfg', root.value), config.value)
    scheduleAuto()
  }

  let autoTimer: ReturnType<typeof setTimeout> | undefined
  function scheduleAuto() {
    clearTimeout(autoTimer)
    if (!config.value.auto || !status.value.initialized || !root.value) return
    autoTimer = setTimeout(() => void autoCheckpoint(), Math.max(1, config.value.minutes) * 60_000)
  }

  async function autoCheckpoint() {
    if (!root.value || !status.value.initialized || !config.value.auto) return
    await refreshStatus()
    // Em merge/conflito, ou sem mudanças, não faz nada.
    if (merging.value || !dirty.value) return
    await checkpoint({ quiet: true })
  }

  /** Checkpoint antes de trocar de projeto / fechar, sem depender do estado reativo atual. */
  async function checkpointRoot(r: string) {
    try {
      const s = await historyStatus(r)
      if (!s.initialized || s.state !== 'clean' || s.entries.length === 0) return
      if (!readJson(key('cfg', r), DEFAULT_CONFIG).auto) return
      if (!(identity.value ?? (await identityGet()))) return
      await historyCheckpoint(r)
    } catch {
      // melhor esforço
    }
  }

  // Cada gravação reinicia a contagem de inatividade.
  watch(
    () => editor.saved,
    () => {
      if (!status.value.initialized) return
      scheduleAuto()
      void refreshStatus()
    },
  )

  watch(
    () => project.root,
    async (next, prev) => {
      clearTimeout(autoTimer)
      if (prev) await checkpointRoot(prev)
      comparison.value = null
      if (next) {
        config.value = readJson(key('cfg', next), { ...DEFAULT_CONFIG })
        declined.value = readJson(key('declined', next), { declined: false }).declined
      }
      await refresh()
      scheduleAuto()
      if (next && !status.value.initialized && !declined.value) {
        const offered = readJson(key('offered', next), { offered: false }).offered
        if (!offered) {
          writeJson(key('offered', next), { offered: true })
          notify.notify('info', t('history.offer'), 9000)
        }
      }
    },
    { immediate: true },
  )

  async function installCloseHook() {
    if (!isTauri()) return
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    await getCurrentWindow().onCloseRequested(async () => {
      // Sem preventDefault: a janela fecha depois que este handler termina.
      await editor.saveAll().catch(() => undefined)
      if (root.value) await checkpointRoot(root.value)
    })
  }

  return {
    status,
    commits,
    hasMore,
    loading,
    busy,
    filters,
    config,
    identity,
    identityDialog,
    declined,
    comparison,
    lastCheckpointAt,
    nameDialog,
    initialized,
    dirty,
    merging,
    refresh,
    refreshStatus,
    reloadLog,
    loadMore,
    ensureIdentity,
    saveIdentity,
    cancelIdentity,
    enable,
    decline,
    checkpoint,
    commitManual,
    stage,
    nameVersion,
    renameVersion,
    removeVersion,
    restore,
    restoreText,
    exportZip,
    exportPdf,
    compare,
    compareWithCurrent,
    compareCommit,
    compareWorkdir,
    closeComparison,
    setConfig,
    installCloseHook,
  }
})

/** Commit "esqueleto" para comparar com um pai que está fora da página carregada. */
function stub(id: string): CommitInfo {
  return {
    id,
    short: id.slice(0, 7),
    parent: null,
    summary: id.slice(0, 7),
    message: '',
    author: '',
    email: '',
    time: 0,
    filesChanged: 0,
    checkpoint: false,
    tags: [],
  }
}
