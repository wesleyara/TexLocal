import { describe, expect, it } from 'vitest'
import ptBR from './pt-BR.json'
import en from './en.json'
import { detectLocale } from './index'

function flatten(obj: Record<string, unknown>, prefix = ''): string[] {
  return Object.entries(obj).flatMap(([k, v]) =>
    typeof v === 'object' && v !== null
      ? flatten(v as Record<string, unknown>, `${prefix}${k}.`)
      : [`${prefix}${k}`],
  )
}

describe('i18n', () => {
  it('pt-BR e en têm exatamente as mesmas chaves', () => {
    const a = flatten(ptBR).sort()
    const b = flatten(en).sort()
    expect(a).toEqual(b)
  })

  it('detecta idioma', () => {
    expect(detectLocale('pt-BR')).toBe('pt-BR')
    expect(detectLocale('pt-PT')).toBe('pt-BR')
    expect(detectLocale('en-US')).toBe('en')
    expect(detectLocale('de')).toBe('en')
  })
})
