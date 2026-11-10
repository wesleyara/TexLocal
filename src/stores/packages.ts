import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { CompileOutcome } from '../compile/api'
import { t } from '../i18n'
import {
  cancelPackageInstall,
  installPackages,
  onPackageProgress,
  resolvePackage,
  toInstallError,
} from '../tex/api'
import { useCompileStore } from './compile'
import { useNotificationsStore } from './notifications'
import { useProjectStore } from './project'
import { useSettingsStore } from './settings'
import { useUiStore } from './ui'

/**
 * Quantas vezes seguidas instalar-e-recompilar antes de desistir (evita laço infinito). O LaTeX
 * para no primeiro arquivo que falta, então classes grandes (ex.: abntex2) pedem várias rodadas.
 */
export const MAX_ROUNDS = 15

export interface Suggestion {
  /** Pacotes do TeX Live a instalar. */
  packages: string[]
  /** Arquivos que o TeX não encontrou. */
  files: string[]
}

/** Detecta pacote faltante na compilação, sugere (ou faz) a instalação e recompila (TL-208/407). */
export const usePackagesStore = defineStore('packages', () => {
  const suggestion = ref<Suggestion | null>(null)
  /** Arquivos que não pertencem a nenhum pacote conhecido (ex.: arquivo do próprio projeto). */
  const unresolved = ref<string[]>([])
  const installing = ref(false)
  const error = ref<string | null>(null)

  /** Pacotes já instalados nesta sequência: se continuam faltando, instalar de novo não ajuda. */
  const attempted = new Set<string>()
  let rounds = 0
  let recompiling = false
  /** O usuário já aprovou uma instalação nesta sequência: as próximas rodadas seguem sozinhas. */
  let approved = false
  let generation = 0

  const names = computed(() => suggestion.value?.packages.join(', ') ?? '')

  function clear() {
    suggestion.value = null
    unresolved.value = []
    error.value = null
  }

  function dismiss() {
    clear()
  }

  /** O `tlmgr` de uma instalação do sistema precisa de `--usermode`; a gerenciada é do usuário. */
  function usermode(): boolean {
    return useSettingsStore().effective.texSource === 'system'
  }

  /** Chamado depois de cada compilação. */
  async function inspect(outcome: CompileOutcome): Promise<void> {
    const settings = useSettingsStore()
    const mine = ++generation
    // Uma compilação que não veio da instalação encerra a sequência anterior.
    if (!recompiling) {
      rounds = 0
      approved = false
      attempted.clear()
    }
    recompiling = false
    clear()

    const mode = settings.effective.texAutoInstall
    const missing = outcome.missingFiles
    if (mode === 'never' || !missing.length || rounds >= MAX_ROUNDS) return

    const files: string[] = []
    const packages: string[] = []
    const lost: string[] = []
    let failure: string | null = null
    for (const m of missing) {
      try {
        const pkg = await resolvePackage(settings.effective.texBinDir, usermode(), m.name)
        if (mine !== generation) return
        if (!pkg) lost.push(m.name)
        else if (!attempted.has(pkg)) {
          files.push(m.name)
          if (!packages.includes(pkg)) packages.push(pkg)
        }
      } catch (e) {
        failure = t(toInstallError(e).i18nKey)
        break
      }
    }
    if (mine !== generation) return

    unresolved.value = lost
    error.value = failure
    if (!packages.length) return
    suggestion.value = { packages, files }

    if (mode === 'auto' || approved) {
      void install()
    } else {
      // O painel de problemas é onde a sugestão aparece.
      useUiStore().layout.bottomVisible = true
    }
  }

  async function install(): Promise<boolean> {
    const current = suggestion.value
    if (!current || installing.value) return false
    const settings = useSettingsStore()
    const notifications = useNotificationsStore()
    const compile = useCompileStore()

    approved = true
    installing.value = true
    error.value = null
    const label = t('packages.installing', { names: current.packages.join(', ') })
    const taskId = notifications.startTask(label, () => void cancelPackageInstall())
    const unlisten = await onPackageProgress((p) => {
      notifications.updateTask(taskId, p.total ? p.current / p.total : null)
    })
    try {
      await installPackages(settings.effective.texBinDir, usermode(), current.packages)
      current.packages.forEach((p) => attempted.add(p))
      notifications.notify(
        'success',
        t('packages.installed', { names: current.packages.join(', ') }),
      )
      suggestion.value = null
      rounds++
      recompiling = true
      installing.value = false
      // O latexmk não reavalia um arquivo que faltava; sem `-g` ele responde "Nothing to do".
      await compile.compile({ manual: false, force: true })
      return true
    } catch (e) {
      const err = toInstallError(e)
      error.value = t(err.i18nKey)
      if (err.kind !== 'cancelled') notifications.notify('error', error.value)
      return false
    } finally {
      unlisten()
      notifications.finishTask(taskId)
      installing.value = false
    }
  }

  // Trocar de projeto descarta a sugestão e o histórico.
  const project = useProjectStore()
  project.$subscribe(
    (_m, s) => {
      if (s.root === null || s.root !== lastRoot) {
        lastRoot = s.root
        clear()
        attempted.clear()
        rounds = 0
        approved = false
      }
    },
    { detached: true },
  )
  let lastRoot: string | null = project.root

  return { suggestion, unresolved, installing, error, names, inspect, install, dismiss }
})
