import { LanguageSupport, StreamLanguage, type StreamParser } from '@codemirror/language'
import { closeBrackets } from '@codemirror/autocomplete'

// ───────────────────────── LaTeX ─────────────────────────

export interface LatexState {
  /** Dentro de math: '$', '$$', ')' (\( \)), ']' (\[ \]) ou ambiente (nome). */
  math: string | null
  /** Dentro de verbatim: nome do ambiente (ou delimitador de \verb). */
  verbatim: string | null
  verbDelim: string | null
  /** Próximo `{...}` é nome de ambiente. */
  envName: boolean
  /** O nome de ambiente em leitura pertence a um egin (e não a um \end). */
  beginPending: boolean
}

const MATH_ENVS = new Set([
  'equation',
  'align',
  'alignat',
  'gather',
  'multline',
  'eqnarray',
  'displaymath',
  'math',
  'flalign',
])
const VERBATIM_ENVS = new Set(['verbatim', 'Verbatim', 'lstlisting', 'minted', 'comment'])

/**
 * Tokenizador de LaTeX (comandos, ambientes, math inline/display, comentários, verbatim, argumentos).
 * Os nomes de token seguem o vocabulário do CodeMirror (`keyword`, `comment`, `tag`...).
 */
export const latexParser: StreamParser<LatexState> = {
  name: 'latex',
  startState: () => ({
    math: null,
    verbatim: null,
    verbDelim: null,
    envName: false,
    beginPending: false,
  }),
  copyState: (s) => ({ ...s }),
  token(stream, state) {
    if (state.verbDelim) {
      const d = state.verbDelim
      while (!stream.eol()) {
        if (stream.next() === d) {
          state.verbDelim = null
          break
        }
      }
      return 'string'
    }
    if (state.verbatim) {
      const end = `\\end{${state.verbatim}}`
      if (stream.match(end)) {
        state.verbatim = null
        return 'keyword'
      }
      // consome até o próximo possível fim
      while (!stream.eol() && !stream.match(end, false)) stream.next()
      return 'string'
    }
    if (stream.eatSpace()) return null

    const ch = stream.peek()
    if (ch === '%') {
      stream.skipToEnd()
      return 'comment'
    }

    if (state.envName) {
      if (stream.eat('{')) return 'bracket'
      if (stream.match(/^[^}\s]+/)) {
        const name = stream.current()
        state.envName = false
        if (state.beginPending) {
          state.beginPending = false
          if (VERBATIM_ENVS.has(name)) state.verbatim = name
          else if (MATH_ENVS.has(name.replace(/\*$/, ''))) state.math = name
        } else if (state.math === name) {
          state.math = null
        }
        return 'tag'
      }
      state.envName = false
    }

    if (ch === '\\') {
      stream.next()
      const n = stream.peek()
      if (n === '(' || n === '[') {
        stream.next()
        if (!state.math) state.math = n === '(' ? ')' : ']'
        return 'keyword'
      }
      if (n === ')' || n === ']') {
        stream.next()
        if (state.math === n) state.math = null
        return 'keyword'
      }
      if (stream.match(/^[A-Za-z@]+\*?/)) {
        const name = stream.current().slice(1)
        if (name === 'begin' || name === 'end') {
          state.envName = true
          state.beginPending = name === 'begin'
        } else if (name === 'verb' || name === 'verb*') {
          const d = stream.next()
          if (d) state.verbDelim = d
        }
        return 'keyword'
      }
      stream.next() // símbolo de controle: \\ \% \{ ...
      return 'keyword'
    }

    if (ch === '$') {
      const double = stream.match('$$', false)
      stream.next()
      if (double) stream.next()
      const kind = double ? '$$' : '$'
      state.math = state.math === kind ? null : (state.math ?? kind)
      return 'keyword'
    }

    if (ch === '{' || ch === '}' || ch === '[' || ch === ']') {
      stream.next()
      return 'bracket'
    }
    if (ch === '&' || ch === '#' || ch === '~') {
      stream.next()
      return 'meta'
    }

    if (state.math) {
      if (stream.match(/^[0-9.]+/)) return 'number'
      if (stream.match(/^[A-Za-z]+/)) return 'atom'
      if (stream.match(/^[_^=+\-*/<>|!]/)) return 'operator'
      stream.next()
      return 'atom'
    }
    // texto comum até o próximo caractere especial
    if (!stream.match(/^[^\\%${}[\]&#~\s]+/)) stream.next()
    return null
  },
  languageData: {
    commentTokens: { line: '%' },
    closeBrackets: { brackets: ['(', '[', '{', '$'] },
    indentOnInput: /^\s*\\end\{[^}]*\}$/,
  },
}

const latexStream = StreamLanguage.define(latexParser)

// ───────────────────────── BibTeX ─────────────────────────

interface BibState {
  /** 0 = fora de entrada, 1 = após @tipo (esperando `{`), 2 = dentro da entrada, 3 = valor */
  mode: 0 | 1 | 2 | 3
  depth: number
  quote: boolean
}

export const bibtexParser: StreamParser<BibState> = {
  name: 'bibtex',
  startState: () => ({ mode: 0, depth: 0, quote: false }),
  copyState: (s) => ({ ...s }),
  token(stream, state) {
    if (state.mode === 0) {
      if (stream.match(/^@[A-Za-z]+/)) {
        state.mode = 1
        return 'keyword'
      }
      if (!stream.skipTo('@')) stream.skipToEnd()
      return 'comment'
    }
    if (stream.eatSpace()) return null
    if (state.mode === 1) {
      if (stream.eat('{') || stream.eat('(')) {
        state.mode = 2
        state.depth = 1
        return 'bracket'
      }
      stream.next()
      return null
    }
    if (state.mode === 2) {
      if (stream.match(/^[^\s=,{}]+(?=\s*,)/)) return 'atom' // chave de citação
      if (stream.match(/^[A-Za-z_-]+(?=\s*=)/)) return 'tag'
      if (stream.eat('=')) {
        state.mode = 3
        return 'operator'
      }
      if (stream.eat('}') || stream.eat(')')) {
        state.mode = 0
        return 'bracket'
      }
      stream.next()
      return null
    }
    // valor
    if (state.depth > 1 || stream.peek() === '{') {
      while (!stream.eol()) {
        const c = stream.next()
        if (c === '{') state.depth++
        else if (c === '}') {
          state.depth--
          if (state.depth === 1) {
            state.mode = 2
            break
          }
        }
      }
      return 'string'
    }
    if (stream.peek() === '"' || state.quote) {
      if (!state.quote) stream.next()
      state.quote = true
      while (!stream.eol()) {
        if (stream.next() === '"') {
          state.quote = false
          state.mode = 2
          break
        }
      }
      return 'string'
    }
    if (stream.match(/^\d+/)) {
      state.mode = 2
      return 'number'
    }
    if (stream.eat(',')) {
      state.mode = 2
      return null
    }
    if (stream.eat('#')) return 'operator'
    if (stream.eat('}')) {
      state.mode = 0
      return 'bracket'
    }
    if (!stream.match(/^[A-Za-z_]+/)) stream.next()
    return 'variableName'
  },
  languageData: { closeBrackets: { brackets: ['{', '"'] } },
}

const bibtexStream = StreamLanguage.define(bibtexParser)

// ───────────────────────── Markdown básico ─────────────────────────

interface MdState {
  fence: boolean
}

export const markdownParser: StreamParser<MdState> = {
  name: 'markdown-basic',
  startState: () => ({ fence: false }),
  copyState: (s) => ({ ...s }),
  token(stream, state) {
    if (stream.sol() && stream.match(/^```/)) {
      state.fence = !state.fence
      stream.skipToEnd()
      return 'string'
    }
    if (state.fence) {
      stream.skipToEnd()
      return 'string'
    }
    if (stream.sol() && stream.match(/^#{1,6}\s.*/)) return 'heading'
    if (stream.match(/^\*\*[^*]+\*\*/)) return 'strong'
    if (stream.match(/^`[^`]+`/)) return 'string'
    if (stream.match(/^\[[^\]]*\]\([^)]*\)/)) return 'link'
    if (stream.sol() && stream.match(/^(\s*[-*+]|\s*\d+\.)\s/)) return 'keyword'
    stream.next()
    return null
  },
}

const markdownStream = StreamLanguage.define(markdownParser)

// ───────────────────────── seleção por arquivo ─────────────────────────

export type LanguageId = 'latex' | 'bibtex' | 'markdown' | 'text'

export function languageIdFor(path: string): LanguageId {
  const ext = /\.([^./\\]+)$/.exec(path)?.[1]?.toLowerCase() ?? ''
  if (['tex', 'sty', 'cls', 'dtx', 'ltx', 'def', 'cfg'].includes(ext)) return 'latex'
  if (ext === 'bib') return 'bibtex'
  if (ext === 'md') return 'markdown'
  return 'text'
}

export function languageFor(path: string) {
  switch (languageIdFor(path)) {
    case 'latex':
      return [new LanguageSupport(latexStream), closeBrackets()]
    case 'bibtex':
      return [new LanguageSupport(bibtexStream), closeBrackets()]
    case 'markdown':
      return [new LanguageSupport(markdownStream)]
    default:
      return []
  }
}
