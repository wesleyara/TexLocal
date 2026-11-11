/** Parser da sintaxe de snippets do VS Code (`$1`, `${1:padrão}`, `${1|a,b|}`, variáveis, regex). */

export type CaseMod = 'upcase' | 'downcase' | 'capitalize' | 'pascalcase' | 'camelcase'

export type FormatPart =
  | { kind: 'text'; value: string }
  | {
      kind: 'group'
      group: number
      mod?: CaseMod
      /** `${1:+se}`, `${1:?se:senão}`, `${1:-senão}`. */
      ifText?: string
      elseText?: string
    }

export interface Transform {
  source: string
  flags: string
  format: FormatPart[]
}

export type SnippetNode =
  | { t: 'text'; value: string }
  | {
      t: 'stop'
      index: number
      children: SnippetNode[]
      choices?: string[]
      transform?: Transform
    }
  | { t: 'var'; name: string; children: SnippetNode[]; transform?: Transform }

const CASE_MODS: CaseMod[] = ['upcase', 'downcase', 'capitalize', 'pascalcase', 'camelcase']

class Parser {
  pos = 0
  constructor(readonly src: string) {}

  peek(): string | undefined {
    return this.src[this.pos]
  }

  /** Texto até um dos terminadores (não consumido); `\` escapa `$`, `}`, `\` e os terminadores. */
  text(stops: string, extraEscapes = ''): string {
    let out = ''
    while (this.pos < this.src.length) {
      const c = this.src[this.pos]
      if (c === '\\') {
        const n = this.src[this.pos + 1]
        if (n !== undefined && ('$}\\' + extraEscapes + stops).includes(n)) {
          out += n
          this.pos += 2
          continue
        }
        out += c
        this.pos++
        continue
      }
      if (stops.includes(c)) break
      if (c === '$' && !stops.includes('$')) break
      out += c
      this.pos++
    }
    return out
  }

  /** Sequência de nós até um dos terminadores (em `stops`). */
  nodes(stops: string): SnippetNode[] {
    const out: SnippetNode[] = []
    while (this.pos < this.src.length) {
      const c = this.src[this.pos]
      if (stops.includes(c)) break
      if (c === '$') {
        const node = this.dollar()
        if (node) {
          out.push(node)
          continue
        }
        out.push({ t: 'text', value: '$' })
        this.pos++
        continue
      }
      const value = this.text(stops)
      if (value) out.push({ t: 'text', value })
      else break
    }
    return out
  }

  int(): number | null {
    const m = /^\d+/.exec(this.src.slice(this.pos))
    if (!m) return null
    this.pos += m[0].length
    return Number(m[0])
  }

  name(): string | null {
    const m = /^[A-Za-z_][A-Za-z0-9_]*/.exec(this.src.slice(this.pos))
    if (!m) return null
    this.pos += m[0].length
    return m[0]
  }

  dollar(): SnippetNode | null {
    const start = this.pos
    this.pos++ // $
    if (this.peek() === '{') {
      this.pos++
      const index = this.int()
      if (index !== null) {
        const node = this.afterIndex(index)
        if (node) return node
      } else {
        const name = this.name()
        if (name) {
          const node = this.afterName(name)
          if (node) return node
        }
      }
      this.pos = start
      return null
    }
    const index = this.int()
    if (index !== null) return { t: 'stop', index, children: [] }
    const name = this.name()
    if (name) return { t: 'var', name, children: [] }
    this.pos = start
    return null
  }

  afterIndex(index: number): SnippetNode | null {
    const c = this.peek()
    if (c === '}') {
      this.pos++
      return { t: 'stop', index, children: [] }
    }
    if (c === ':') {
      this.pos++
      const children = this.nodes('}')
      if (this.peek() !== '}') return null
      this.pos++
      return { t: 'stop', index, children }
    }
    if (c === '|') {
      this.pos++
      const choices: string[] = []
      for (;;) {
        choices.push(this.text(',|', ','))
        if (this.peek() === ',') {
          this.pos++
          continue
        }
        break
      }
      if (this.peek() !== '|' || this.src[this.pos + 1] !== '}') return null
      this.pos += 2
      return {
        t: 'stop',
        index,
        children: choices[0] ? [{ t: 'text', value: choices[0] }] : [],
        choices,
      }
    }
    if (c === '/') {
      const transform = this.transform()
      if (!transform || this.peek() !== '}') return null
      this.pos++
      return { t: 'stop', index, children: [], transform }
    }
    return null
  }

  afterName(name: string): SnippetNode | null {
    const c = this.peek()
    if (c === '}') {
      this.pos++
      return { t: 'var', name, children: [] }
    }
    if (c === ':') {
      this.pos++
      const children = this.nodes('}')
      if (this.peek() !== '}') return null
      this.pos++
      return { t: 'var', name, children }
    }
    if (c === '/') {
      const transform = this.transform()
      if (!transform || this.peek() !== '}') return null
      this.pos++
      return { t: 'var', name, children: [], transform }
    }
    return null
  }

  /** `/regex/formato/flags` (a posição está no primeiro `/`). */
  transform(): Transform | null {
    this.pos++ // /
    let source = ''
    while (this.pos < this.src.length && this.src[this.pos] !== '/') {
      if (this.src[this.pos] === '\\' && this.pos + 1 < this.src.length) {
        source += this.src[this.pos] + this.src[this.pos + 1]
        this.pos += 2
      } else {
        source += this.src[this.pos++]
      }
    }
    if (this.peek() !== '/') return null
    this.pos++
    const format: FormatPart[] = []
    while (this.pos < this.src.length && this.src[this.pos] !== '/') {
      if (this.src[this.pos] === '$') {
        const part = this.formatGroup()
        if (part) {
          format.push(part)
          continue
        }
        format.push({ kind: 'text', value: '$' })
        this.pos++
        continue
      }
      const value = this.text('/')
      if (value) format.push({ kind: 'text', value })
      else break
    }
    if (this.peek() !== '/') return null
    this.pos++
    const flags = /^[a-z]*/.exec(this.src.slice(this.pos))?.[0] ?? ''
    this.pos += flags.length
    return { source, flags, format }
  }

  formatGroup(): FormatPart | null {
    const start = this.pos
    this.pos++
    if (this.peek() !== '{') {
      const group = this.int()
      if (group !== null) return { kind: 'group', group }
      this.pos = start
      return null
    }
    this.pos++
    const group = this.int()
    if (group === null) {
      this.pos = start
      return null
    }
    if (this.peek() === '}') {
      this.pos++
      return { kind: 'group', group }
    }
    if (this.peek() !== ':') {
      this.pos = start
      return null
    }
    this.pos++
    const mod = CASE_MODS.find((m) => this.src.startsWith(`/${m}}`, this.pos))
    if (mod) {
      this.pos += mod.length + 2
      return { kind: 'group', group, mod }
    }
    const c = this.peek()
    if (c === '+' || c === '-' || c === '?') {
      this.pos++
      const first = this.text(c === '?' ? ':}' : '}', ':')
      let second: string | undefined
      if (c === '?' && this.peek() === ':') {
        this.pos++
        second = this.text('}')
      }
      if (this.peek() !== '}') {
        this.pos = start
        return null
      }
      this.pos++
      if (c === '+') return { kind: 'group', group, ifText: first }
      if (c === '-') return { kind: 'group', group, elseText: first }
      return { kind: 'group', group, ifText: first, elseText: second ?? '' }
    }
    const elseText = this.text('}')
    if (this.peek() !== '}') {
      this.pos = start
      return null
    }
    this.pos++
    return { kind: 'group', group, elseText }
  }
}

/** Converte o corpo do snippet em árvore. Nunca lança: trechos inválidos viram texto. */
export function parseSnippet(body: string): SnippetNode[] {
  const p = new Parser(body)
  const out: SnippetNode[] = []
  while (p.pos < body.length) {
    const nodes = p.nodes('')
    out.push(...nodes)
    if (p.pos < body.length && nodes.length === 0) {
      // `}` solto ou outro caractere que nenhum ramo consumiu
      out.push({ t: 'text', value: body[p.pos] })
      p.pos++
    }
  }
  return mergeText(out)
}

function mergeText(nodes: SnippetNode[]): SnippetNode[] {
  const out: SnippetNode[] = []
  for (const n of nodes) {
    const last = out.at(-1)
    if (n.t === 'text' && last?.t === 'text') last.value += n.value
    else out.push(n)
  }
  return out
}

/** O corpo usa a variável `name` em algum ponto? */
export function usesVariable(nodes: SnippetNode[], name: string): boolean {
  return nodes.some(
    (n) =>
      (n.t === 'var' && (n.name === name || usesVariable(n.children, name))) ||
      (n.t === 'stop' && usesVariable(n.children, name)),
  )
}

export function applyCase(text: string, mod: CaseMod): string {
  const words = text.split(/[^A-Za-z0-9À-ɏ]+/).filter(Boolean)
  const cap = (w: string) => w.charAt(0).toUpperCase() + w.slice(1)
  switch (mod) {
    case 'upcase':
      return text.toUpperCase()
    case 'downcase':
      return text.toLowerCase()
    case 'capitalize':
      return cap(text)
    case 'pascalcase':
      return words.map((w) => cap(w)).join('')
    case 'camelcase':
      return words
        .map((w, i) => (i === 0 ? w.charAt(0).toLowerCase() + w.slice(1) : cap(w)))
        .join('')
  }
}

/** Aplica a transformação regex a `input`. Regex inválida devolve o texto original. */
export function applyTransform(input: string, tr: Transform): string {
  let re: RegExp
  try {
    re = new RegExp(tr.source, tr.flags.replace(/[^dgimsuy]/g, ''))
  } catch {
    return input
  }
  return input.replace(re, (...args: unknown[]) => {
    const groupsIdx = args.findIndex((a) => typeof a === 'number')
    const groups = args.slice(0, groupsIdx) as (string | undefined)[]
    let out = ''
    for (const part of tr.format) {
      if (part.kind === 'text') {
        out += part.value
        continue
      }
      const g = groups[part.group]
      if (g) {
        if (part.mod) out += applyCase(g, part.mod)
        else if (part.ifText !== undefined) out += part.ifText
        else out += g
      } else if (part.elseText !== undefined) out += part.elseText
    }
    return out
  })
}
