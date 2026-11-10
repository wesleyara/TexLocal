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

describe('explicações de erros de compilação', () => {
  it('toda explicação emitida pelo parser Rust existe nos dois idiomas', async () => {
    const { readFileSync } = await import('node:fs')
    const { resolve } = await import('node:path')
    const rust = (file: string) =>
      readFileSync(resolve(process.cwd(), 'src-tauri/crates/texlocal-tex/src', file), 'utf8')
    const ids = new Set<string>()
    // Tabela `("trecho", "id"),` e retornos diretos `Some("id")`.
    for (const m of rust('diagnostics.rs').matchAll(/",\s*"([a-z][A-Za-z]+)"\),/g)) ids.add(m[1])
    for (const m of rust('diagnostics.rs').matchAll(/return Some\("([a-z][A-Za-z]+)"\)/g))
      ids.add(m[1])
    for (const m of rust('compile.rs').matchAll(/explanation: Some\("([a-z][A-Za-z]+)"\)/g)) {
      ids.add(m[1])
    }
    expect(ids.size).toBeGreaterThan(20)
    const explain = (l: unknown) =>
      Object.keys((l as { compile: { explain: object } }).compile.explain)
    for (const id of ids) {
      expect(explain(ptBR), `falta pt-BR compile.explain.${id}`).toContain(id)
      expect(explain(en), `falta en compile.explain.${id}`).toContain(id)
    }
  })
})
