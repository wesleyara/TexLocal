import { describe, expect, it } from 'vitest'
import {
  eventToBinding,
  findConflicts,
  formatBinding,
  fuzzyScore,
  matchesEvent,
  normalizeBinding,
  type Command,
} from './registry'

const ev = (
  key: string,
  o: Partial<Record<'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey', boolean>> = {},
) => ({
  key,
  ctrlKey: false,
  metaKey: false,
  shiftKey: false,
  altKey: false,
  ...o,
})

describe('keybindings', () => {
  it('normaliza a ordem dos modificadores', () => {
    expect(normalizeBinding('Shift+Mod+P')).toBe('mod+shift+p')
  })

  it('Mod é Ctrl fora do macOS e Cmd no macOS', () => {
    expect(matchesEvent('Mod+B', ev('b', { ctrlKey: true }), false)).toBe(true)
    expect(matchesEvent('Mod+B', ev('b', { metaKey: true }), false)).toBe(false)
    expect(matchesEvent('Mod+B', ev('b', { metaKey: true }), true)).toBe(true)
  })

  it('ignora eventos só de modificador', () => {
    expect(eventToBinding(ev('Control', { ctrlKey: true }), false)).toBeNull()
  })

  it('formata por plataforma', () => {
    expect(formatBinding('Mod+Shift+P', false)).toBe('Ctrl+Shift+P')
    expect(formatBinding('Mod+Shift+P', true)).toBe('⌘⇧P')
  })

  it('detecta conflitos, inclusive por override', () => {
    const cmds: Command[] = [
      { id: 'a', titleKey: 'a', keybinding: 'Mod+B', run: () => {} },
      { id: 'b', titleKey: 'b', keybinding: 'Mod+J', run: () => {} },
    ]
    expect(findConflicts(cmds)).toEqual([])
    expect(findConflicts(cmds, { b: 'mod+b' })).toEqual([['a', 'b']])
  })
})

describe('fuzzyScore', () => {
  it('casa subsequências e rejeita o resto', () => {
    expect(fuzzyScore('tgs', 'Toggle Sidebar')).not.toBeNull()
    expect(fuzzyScore('xyz', 'Toggle Sidebar')).toBeNull()
  })

  it('prefere casamentos consecutivos', () => {
    const a = fuzzyScore('side', 'Toggle Sidebar') ?? 0
    const b = fuzzyScore('side', 'Switch input device') ?? 0
    expect(a).toBeGreaterThan(b)
  })
})
