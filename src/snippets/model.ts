import { fuzzyScore } from '../commands/registry'

export type SnippetScope = 'builtin' | 'global' | 'project'

export interface Snippet {
  /** Chave única: `escopo:arquivo:nome`. */
  id: string
  /** Nome do snippet no JSON (chave do objeto). */
  name: string
  prefixes: string[]
  body: string
  description: string
  /** Linguagens (`latex`, `bibtex`…); vazio = todas. */
  languages: string[]
  scope: SnippetScope
  /** Categoria (arquivo do padrão embutido); usada para desativar em bloco. */
  category: string
  /** Arquivo de origem (caminho absoluto; vazio no padrão embutido). */
  file: string
  /** Atalho opcional (`keybinding` no JSON; ignorado pelo VS Code). */
  keybinding?: string
}

export interface SnippetProblem {
  file: string
  message: string
  /** 0-based, quando conhecido. */
  line?: number
}

export interface ParsedFile {
  snippets: Snippet[]
  problems: SnippetProblem[]
}

/** Remove comentários `//` e `/* *\/` e vírgulas finais (JSONC, como nos arquivos do VS Code). */
export function stripJsonc(text: string): string {
  let out = ''
  let i = 0
  let inString = false
  while (i < text.length) {
    const c = text[i]
    if (inString) {
      out += c
      if (c === '\\') {
        out += text[i + 1] ?? ''
        i += 2
        continue
      }
      if (c === '"') inString = false
      i++
      continue
    }
    if (c === '"') {
      inString = true
      out += c
      i++
    } else if (c === '/' && text[i + 1] === '/') {
      while (i < text.length && text[i] !== '\n') i++
    } else if (c === '/' && text[i + 1] === '*') {
      i += 2
      while (i < text.length && !(text[i] === '*' && text[i + 1] === '/')) {
        if (text[i] === '\n') out += '\n'
        i++
      }
      i += 2
    } else {
      out += c
      i++
    }
  }
  return out.replace(/,(\s*[}\]])/g, '$1')
}

function lineOfError(message: string, text: string): number | undefined {
  const pos = /position (\d+)/.exec(message)?.[1]
  if (pos !== undefined) return text.slice(0, Number(pos)).split('\n').length - 1
  const line = /line (\d+)/.exec(message)?.[1]
  return line ? Number(line) - 1 : undefined
}

function asList(v: unknown): string[] | null {
  if (typeof v === 'string') return [v]
  if (Array.isArray(v) && v.every((x) => typeof x === 'string')) return v as string[]
  return null
}

/** Lê um arquivo de snippets no formato do VS Code. Nunca lança: erros viram `problems`. */
export function parseSnippetFile(
  text: string,
  meta: { scope: SnippetScope; category: string; file: string },
): ParsedFile {
  const problems: SnippetProblem[] = []
  const snippets: Snippet[] = []
  const label = meta.file || meta.category
  let data: unknown
  try {
    data = JSON.parse(stripJsonc(text))
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e)
    return { snippets, problems: [{ file: label, message, line: lineOfError(message, text) }] }
  }
  if (typeof data !== 'object' || data === null || Array.isArray(data)) {
    return { snippets, problems: [{ file: label, message: 'o arquivo deve ser um objeto JSON' }] }
  }
  for (const [name, raw] of Object.entries(data as Record<string, unknown>)) {
    if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) {
      problems.push({ file: label, message: `${name}: definição deve ser um objeto` })
      continue
    }
    const o = raw as Record<string, unknown>
    const prefixes = asList(o.prefix)
    const body = typeof o.body === 'string' ? o.body : asList(o.body)?.join('\n')
    if (!prefixes?.length || prefixes.some((p) => !p)) {
      problems.push({ file: label, message: `${name}: "prefix" ausente ou inválido` })
      continue
    }
    if (body === undefined) {
      problems.push({ file: label, message: `${name}: "body" ausente ou inválido` })
      continue
    }
    const languages = (asList(o.scope) ?? [])
      .flatMap((s) => s.split(','))
      .map((s) => s.trim())
      .filter(Boolean)
    snippets.push({
      id: `${meta.scope}:${label}:${name}`,
      name,
      prefixes,
      body,
      description: typeof o.description === 'string' ? o.description : '',
      languages,
      scope: meta.scope,
      category: meta.category,
      file: meta.file,
      keybinding: typeof o.keybinding === 'string' ? o.keybinding : undefined,
    })
  }
  return { snippets, problems }
}

/** Definição no formato do VS Code (`body` em linhas, `scope` em texto). */
export function snippetToRaw(s: Snippet): Record<string, unknown> {
  return {
    prefix: s.prefixes.length === 1 ? s.prefixes[0] : s.prefixes,
    body: s.body.split('\n'),
    ...(s.description ? { description: s.description } : {}),
    ...(s.languages.length ? { scope: s.languages.join(',') } : {}),
    ...(s.keybinding ? { keybinding: s.keybinding } : {}),
  }
}

export function toVsCodeJson(snippets: Snippet[]): string {
  const out: Record<string, unknown> = {}
  for (const s of snippets) out[s.name] = snippetToRaw(s)
  return JSON.stringify(out, null, 2) + '\n'
}

const RANK: Record<SnippetScope, number> = { builtin: 0, global: 1, project: 2 }

/**
 * Funde as camadas: padrão embutido → global → projeto. No mesmo prefixo (e mesma linguagem)
 * o escopo mais específico vence; um snippet com vários prefixos perde só o prefixo coberto.
 */
export function mergeSnippets(layers: Snippet[][]): Snippet[] {
  const all = layers.flat().sort((a, b) => RANK[a.scope] - RANK[b.scope])
  const winner = new Map<string, Snippet>()
  for (const s of all) {
    for (const p of s.prefixes) {
      for (const lang of s.languages.length ? s.languages : ['*']) {
        winner.set(`${lang}\u0000${p}`, s)
      }
    }
  }
  const kept = new Set<Snippet>()
  const result: Snippet[] = []
  for (const s of all) {
    const live = s.prefixes.filter((p) =>
      (s.languages.length ? s.languages : ['*']).some((l) => winner.get(`${l}\u0000${p}`) === s),
    )
    if (!live.length || kept.has(s)) continue
    kept.add(s)
    result.push(live.length === s.prefixes.length ? s : { ...s, prefixes: live })
  }
  return result
}

/** O snippet vale para a linguagem `lang`? */
export function appliesTo(s: Snippet, lang: string): boolean {
  return !s.languages.length || s.languages.includes(lang)
}

export function searchSnippets(list: Snippet[], query: string): Snippet[] {
  const q = query.trim()
  if (!q) return list
  return list
    .map((s) => {
      const scores = [s.name, s.description, ...s.prefixes]
        .map((t) => fuzzyScore(q, t))
        .filter((x): x is number => x !== null)
      return { s, score: scores.length ? Math.max(...scores) : null }
    })
    .filter((x): x is { s: Snippet; score: number } => x.score !== null)
    .sort((a, b) => b.score - a.score)
    .map((x) => x.s)
}
