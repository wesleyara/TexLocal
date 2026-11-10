import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { CompileOutcome, Manifest } from '../compile/api'

const capi = vi.hoisted(() => ({
  compileStart: vi.fn(),
  compileCancel: vi.fn(),
  getManifest: vi.fn(),
  saveManifest: vi.fn(),
  synctexForward: vi.fn(),
  synctexInverse: vi.fn(),
  toCompileError: (e: unknown) => e,
}))
vi.mock('../compile/api', () => capi)

vi.mock('../tex/api', async (orig) => {
  const actual = await orig<typeof import('../tex/api')>()
  return {
    ...actual,
    resolvePackage: vi.fn(),
    installPackages: vi.fn(),
    cancelPackageInstall: vi.fn(async () => {}),
    onPackageProgress: vi.fn(async () => () => {}),
  }
})

import * as tex from '../tex/api'
import { useCompileStore } from './compile'
import { MAX_ROUNDS, usePackagesStore } from './packages'
import { useProjectStore } from './project'
import { useSettingsStore } from './settings'
import { useUiStore } from './ui'

const manifest: Manifest = {
  project: { main: 'main.tex' },
  tex: { engine: 'pdflatex', packages: [] },
  compile: {
    auto: true,
    shell_escape: false,
    outdir: '.texlocal/build',
    timeout_secs: 300,
    debounce_ms: 1500,
  },
}

function outcome(files: string[]): CompileOutcome {
  return {
    status: files.length ? 'failed' : 'success',
    pdf: null,
    pdfUpdated: false,
    synctex: null,
    log: '',
    output: '',
    diagnostics: [],
    missingFiles: files.map((name) => ({ name, kind: 'cls' })),
    durationMs: 1,
  }
}

async function setup(mode: 'ask' | 'auto' | 'never' = 'ask') {
  capi.getManifest.mockResolvedValue(manifest)
  useProjectStore().root = '/p'
  const compile = useCompileStore()
  await vi.waitFor(() => expect(compile.manifest).not.toBeNull())
  await useSettingsStore().update({ texAutoInstall: mode })
  return { compile, packages: usePackagesStore() }
}

describe('packages store', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('pergunta: sugere o pacote e abre o painel de problemas, sem instalar', async () => {
    vi.mocked(tex.resolvePackage).mockResolvedValue('abntex2')
    capi.compileStart.mockResolvedValue(outcome(['abntex2.cls']))
    const { compile, packages } = await setup('ask')
    await compile.compile({ manual: true, save: false })
    await vi.waitFor(() => expect(packages.suggestion).not.toBeNull())
    expect(packages.suggestion).toEqual({ packages: ['abntex2'], files: ['abntex2.cls'] })
    expect(useUiStore().layout.bottomVisible).toBe(true)
    expect(tex.installPackages).not.toHaveBeenCalled()
  })

  it('"nunca" não resolve nem sugere nada', async () => {
    capi.compileStart.mockResolvedValue(outcome(['abntex2.cls']))
    const { compile, packages } = await setup('never')
    await compile.compile({ manual: true, save: false })
    await Promise.resolve()
    expect(packages.suggestion).toBeNull()
    expect(tex.resolvePackage).not.toHaveBeenCalled()
  })

  it('arquivo sem pacote conhecido vai para "não resolvidos"', async () => {
    vi.mocked(tex.resolvePackage).mockResolvedValue(null)
    capi.compileStart.mockResolvedValue(outcome(['meu.sty']))
    const { compile, packages } = await setup('ask')
    await compile.compile({ manual: true, save: false })
    await vi.waitFor(() => expect(packages.unresolved).toEqual(['meu.sty']))
    expect(packages.suggestion).toBeNull()
  })

  it('instalar recompila; depois do sucesso a sugestão some', async () => {
    vi.mocked(tex.resolvePackage).mockResolvedValue('abntex2')
    vi.mocked(tex.installPackages).mockResolvedValue()
    capi.compileStart
      .mockResolvedValueOnce(outcome(['abntex2.cls']))
      .mockResolvedValueOnce(outcome([]))
    const { compile, packages } = await setup('ask')
    await compile.compile({ manual: true, save: false })
    await vi.waitFor(() => expect(packages.suggestion).not.toBeNull())
    expect(await packages.install()).toBe(true)
    expect(tex.installPackages).toHaveBeenCalledWith(undefined, false, ['abntex2'])
    expect(capi.compileStart).toHaveBeenCalledTimes(2)
    // A primeira compilação é normal; a recompilação depois da instalação força o latexmk.
    expect(capi.compileStart.mock.calls[0][2]).toBeFalsy()
    expect(capi.compileStart.mock.calls[1][2]).toBe(true)
    expect(packages.suggestion).toBeNull()
  })

  it('automático instala sozinho e não repete o mesmo pacote que continua faltando', async () => {
    vi.mocked(tex.resolvePackage).mockResolvedValue('abntex2')
    vi.mocked(tex.installPackages).mockResolvedValue()
    capi.compileStart.mockResolvedValue(outcome(['abntex2.cls']))
    const { compile, packages } = await setup('auto')
    await compile.compile({ manual: true, save: false })
    await vi.waitFor(() => expect(capi.compileStart).toHaveBeenCalledTimes(2))
    await vi.waitFor(() => expect(packages.installing).toBe(false))
    expect(tex.installPackages).toHaveBeenCalledTimes(1)
    expect(packages.suggestion).toBeNull()
  })

  it('depois de aprovar a primeira instalação, as próximas rodadas seguem sozinhas', async () => {
    let n = 0
    vi.mocked(tex.resolvePackage).mockImplementation(async () => `pkg${n}`)
    vi.mocked(tex.installPackages).mockImplementation(async () => {
      n++
    })
    // Três arquivos faltando em sequência (um por compilação) e depois sucesso.
    capi.compileStart
      .mockResolvedValueOnce(outcome(['a.sty']))
      .mockResolvedValueOnce(outcome(['b.sty']))
      .mockResolvedValueOnce(outcome(['c.sty']))
      .mockResolvedValue(outcome([]))
    const { compile, packages } = await setup('ask')
    await compile.compile({ manual: true, save: false })
    await vi.waitFor(() => expect(packages.suggestion).not.toBeNull())
    await packages.install()
    await vi.waitFor(() => expect(capi.compileStart).toHaveBeenCalledTimes(4))
    await vi.waitFor(() => expect(packages.installing).toBe(false))
    expect(tex.installPackages).toHaveBeenCalledTimes(3)
    expect(packages.suggestion).toBeNull()
  })

  it('limita as rodadas de instalar e recompilar', async () => {
    let n = 0
    vi.mocked(tex.resolvePackage).mockImplementation(async () => `pkg${n++}`)
    vi.mocked(tex.installPackages).mockResolvedValue()
    capi.compileStart.mockImplementation(async () => outcome([`f${n}.sty`]))
    const { compile } = await setup('auto')
    await compile.compile({ manual: true, save: false })
    await vi.waitFor(() => expect(capi.compileStart.mock.calls.length).toBeGreaterThan(1))
    await new Promise((r) => setTimeout(r, 50))
    expect(tex.installPackages).toHaveBeenCalledTimes(MAX_ROUNDS)
  })

  it('falha de rede mantém a sugestão e mostra a mensagem amigável', async () => {
    vi.mocked(tex.resolvePackage).mockResolvedValue('abntex2')
    vi.mocked(tex.installPackages).mockRejectedValue({
      kind: 'offline',
      i18nKey: 'tlmgr.error.offline',
      details: '',
    })
    capi.compileStart.mockResolvedValue(outcome(['abntex2.cls']))
    const { compile, packages } = await setup('ask')
    await compile.compile({ manual: true, save: false })
    await vi.waitFor(() => expect(packages.suggestion).not.toBeNull())
    expect(await packages.install()).toBe(false)
    expect(packages.suggestion).not.toBeNull()
    expect(packages.error).toMatch(/repository|repositório/i)
  })
})
