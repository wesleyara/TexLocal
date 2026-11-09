import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useSettingsStore } from './settings'

describe('settings store', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('persiste e recarrega', async () => {
    const s = useSettingsStore()
    expect(await s.update({ theme: 'light' })).toBe(true)
    setActivePinia(createPinia())
    const s2 = useSettingsStore()
    await s2.load()
    expect(s2.effective.theme).toBe('light')
  })

  it('override de projeto vence o global', async () => {
    const s = useSettingsStore()
    await s.update({ theme: 'light' })
    s.project = { theme: 'dark' }
    expect(s.effective.theme).toBe('dark')
  })
})
