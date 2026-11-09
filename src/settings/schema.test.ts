import { describe, expect, it } from 'vitest'
import { DEFAULT_SETTINGS, resolveSettings, validateSettings } from './schema'

describe('validateSettings', () => {
  it('preenche padrões', () => {
    expect(validateSettings({})).toEqual(DEFAULT_SETTINGS)
  })

  it('rejeita valores e chaves inválidos', () => {
    expect(() => validateSettings({ theme: 'neon' })).toThrow(/theme/)
    expect(() => validateSettings({ foo: 1 })).toThrow(/desconhecida/)
    expect(() => validateSettings({ keybindings: { a: 1 } })).toThrow(/keybindings/)
    expect(() => validateSettings([])).toThrow()
  })
})

describe('resolveSettings', () => {
  it('projeto > global > padrão', () => {
    const r = resolveSettings(
      DEFAULT_SETTINGS,
      { theme: 'light', keybindings: { a: 'mod+a' } },
      { theme: 'system', keybindings: { b: 'mod+b' } },
    )
    expect(r.theme).toBe('system')
    expect(r.locale).toBe('auto')
    expect(r.keybindings).toEqual({ a: 'mod+a', b: 'mod+b' })
  })
})
