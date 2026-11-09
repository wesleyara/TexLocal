import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

vi.mock('../tex/api', async (orig) => {
  const actual = await orig<typeof import('../tex/api')>()
  return {
    ...actual,
    installTex: vi.fn(),
    cancelInstall: vi.fn(async () => {}),
    onInstallEvent: vi.fn(async () => () => {}),
  }
})

import * as api from '../tex/api'
import { useNotificationsStore } from './notifications'
import { useSettingsStore } from './settings'
import { useTexInstallStore } from './texInstall'

describe('tex install store', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('instalação bem-sucedida grava a origem gerenciada e notifica', async () => {
    vi.mocked(api.installTex).mockResolvedValue({ year: 2026, tree: '/t', binDir: '/t/bin' })
    const store = useTexInstallStore()
    expect(await store.start('minimal')).toBe(true)
    expect(store.status).toBe('done')
    const settings = useSettingsStore()
    expect(settings.effective.texSource).toBe('managed')
    expect(settings.effective.texBinDir).toBe('/t/bin')
    expect(useNotificationsStore().tasks).toHaveLength(0)
    expect(useNotificationsStore().history[0].kind).toBe('success')
  })

  it('falha expõe a chave de tradução e não altera as configurações', async () => {
    vi.mocked(api.installTex).mockRejectedValue({
      kind: 'network',
      i18nKey: 'installer.error.network',
      details: 'x',
    })
    const store = useTexInstallStore()
    expect(await store.start('minimal')).toBe(false)
    expect(store.status).toBe('error')
    expect(store.error?.i18nKey).toBe('installer.error.network')
    expect(useSettingsStore().effective.texSource).toBe('unset')
  })

  it('cancelamento não gera notificação de erro', async () => {
    vi.mocked(api.installTex).mockRejectedValue({
      kind: 'cancelled',
      i18nKey: 'installer.error.cancelled',
      details: '',
    })
    const store = useTexInstallStore()
    await store.start('minimal')
    expect(useNotificationsStore().history).toHaveLength(0)
  })

  it('repassa o espelho fixado e não inicia duas instalações', async () => {
    let release: (v: api.InstallResult) => void = () => {}
    vi.mocked(api.installTex).mockReturnValue(new Promise((r) => (release = r)))
    const settings = useSettingsStore()
    await settings.update({ texMirror: 'https://m.example/tlnet' })
    const store = useTexInstallStore()
    const first = store.start('recommended')
    await Promise.resolve()
    expect(await store.start('minimal')).toBe(false)
    release({ year: 2026, tree: '/t', binDir: '/b' })
    await first
    expect(api.installTex).toHaveBeenCalledWith('recommended', 'https://m.example/tlnet')
  })
})
