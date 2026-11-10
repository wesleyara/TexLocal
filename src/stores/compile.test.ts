import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { CompileOutcome, Manifest } from '../compile/api'

const api = vi.hoisted(() => ({
  compileStart: vi.fn(),
  compileCancel: vi.fn(),
  getManifest: vi.fn(),
  saveManifest: vi.fn(),
  synctexForward: vi.fn(),
  synctexInverse: vi.fn(),
  toCompileError: (e: unknown) => e,
}))
vi.mock('../compile/api', () => api)

const ask = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask }))

import { useCompileStore } from './compile'
import { useEditorStore } from './editor'
import { useProjectStore } from './project'

function manifest(over: Partial<Manifest['compile']> = {}): Manifest {
  return {
    project: { main: 'main.tex' },
    tex: { engine: 'pdflatex', packages: [] },
    compile: {
      auto: true,
      shell_escape: false,
      outdir: '.texlocal/build',
      timeout_secs: 300,
      debounce_ms: 1500,
      ...over,
    },
  }
}

function outcome(over: Partial<CompileOutcome> = {}): CompileOutcome {
  return {
    status: 'success',
    pdf: '/p/.texlocal/build/main.pdf',
    pdfUpdated: true,
    synctex: null,
    log: '',
    output: '',
    diagnostics: [],
    missingFiles: [],
    durationMs: 10,
    ...over,
  }
}

async function setup(m: Manifest = manifest()) {
  api.getManifest.mockResolvedValue(m)
  api.saveManifest.mockResolvedValue(undefined)
  const project = useProjectStore()
  project.root = '/p'
  const compile = useCompileStore()
  await vi.waitFor(() => expect(compile.manifest).not.toBeNull())
  return { compile, editor: useEditorStore() }
}

const touch = (editor: ReturnType<typeof useEditorStore>, path = '/p/main.tex') => {
  editor.change = { tick: editor.change.tick + 1, path }
}

describe('compile store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    Object.values(api).forEach((f) => typeof f === 'function' && 'mockReset' in f && f.mockReset())
    ask.mockReset()
  })
  afterEach(() => vi.useRealTimers())

  it('compila, guarda o resultado e avisa o visualizador só quando o PDF mudou', async () => {
    const { compile } = await setup()
    api.compileStart.mockResolvedValueOnce(outcome())
    await compile.compile({ manual: true })
    expect(api.compileStart).toHaveBeenCalledWith('/p', undefined)
    expect(compile.state).toBe('success')
    expect(compile.pdfVersion).toBe(1)

    api.compileStart.mockResolvedValueOnce(outcome({ status: 'failed', pdfUpdated: false }))
    await compile.compile({ manual: true })
    expect(compile.state).toBe('failed')
    expect(compile.pdfVersion).toBe(1)
  })

  it('o caminho esperado do PDF segue o manifesto', async () => {
    const { compile } = await setup(manifest())
    expect(compile.pdfPath).toMatch(/\.texlocal[\\/]build[\\/]main\.pdf$/)
  })

  it('nunca roda duas compilações em paralelo: enfileira uma nova', async () => {
    const { compile } = await setup()
    let release!: (o: CompileOutcome) => void
    api.compileStart
      .mockImplementationOnce(() => new Promise<CompileOutcome>((r) => (release = r)))
      .mockResolvedValueOnce(outcome())
    const first = compile.compile({ manual: true })
    await vi.waitFor(() => expect(compile.state).toBe('running'))
    await compile.compile({ manual: true })
    await compile.compile({ manual: true })
    expect(api.compileStart).toHaveBeenCalledTimes(1)
    expect(compile.queued).toBe(true)

    release(outcome())
    await first
    await vi.waitFor(() => expect(api.compileStart).toHaveBeenCalledTimes(2))
  })

  it('falhas do serviço viram estado de erro', async () => {
    const { compile } = await setup()
    api.compileStart.mockRejectedValueOnce({
      kind: 'latexmkNotFound',
      i18nKey: 'compile.error.latexmkNotFound',
      details: '',
    })
    await compile.compile({ manual: true })
    expect(compile.state).toBe('error')
    expect(compile.error?.kind).toBe('latexmkNotFound')
  })

  it('compilação automática espera a pausa de digitação (debounce do manifesto)', async () => {
    vi.useFakeTimers()
    const { compile, editor } = await setup(manifest({ debounce_ms: 2000 }))
    api.compileStart.mockResolvedValue(outcome())

    touch(editor)
    await vi.advanceTimersByTimeAsync(1500)
    touch(editor) // nova digitação reinicia a contagem
    await vi.advanceTimersByTimeAsync(1999)
    expect(api.compileStart).not.toHaveBeenCalled()
    await vi.advanceTimersByTimeAsync(2)
    expect(api.compileStart).toHaveBeenCalledTimes(1)
    expect(compile.state).toBe('success')
  })

  it('não recompila por arquivos que não são fonte nem com a opção desligada', async () => {
    vi.useFakeTimers()
    const { compile, editor } = await setup()
    api.compileStart.mockResolvedValue(outcome())
    touch(editor, '/p/notes.md')
    await vi.advanceTimersByTimeAsync(5000)
    expect(api.compileStart).not.toHaveBeenCalled()

    await compile.setAuto(false)
    expect(api.saveManifest).toHaveBeenCalled()
    touch(editor)
    await vi.advanceTimersByTimeAsync(5000)
    expect(api.compileStart).not.toHaveBeenCalled()
  })

  it('ligar o shell-escape exige confirmação; desligar não', async () => {
    const { compile } = await setup()
    ask.mockResolvedValueOnce(false)
    expect(await compile.setShellEscape(true)).toBe(false)
    expect(compile.shellEscape).toBe(false)
    expect(api.saveManifest).not.toHaveBeenCalled()

    ask.mockResolvedValueOnce(true)
    expect(await compile.setShellEscape(true)).toBe(true)
    expect(compile.shellEscape).toBe(true)
    expect(api.saveManifest.mock.calls[0][1].compile.shell_escape).toBe(true)

    expect(await compile.setShellEscape(false)).toBe(true)
    expect(ask).toHaveBeenCalledTimes(2) // só as duas tentativas de ligar
  })

  it('busca direta guarda o alvo do destaque; sem posição não faz nada', async () => {
    const { compile, editor } = await setup()
    editor.active = '/p/main.tex'
    api.synctexForward.mockResolvedValueOnce({ page: 2, rects: [{ x: 1, y: 2, w: 3, h: 4 }] })
    expect(await compile.syncToPdf()).toBe(true)
    expect(compile.syncTarget?.page).toBe(2)

    api.synctexForward.mockResolvedValueOnce(null)
    expect(await compile.syncToPdf()).toBe(false)
  })
})
