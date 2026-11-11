import { applyTransform, parseSnippet, type SnippetNode, type Transform } from './parser'

/** Contexto para resolver as variáveis (`$TM_FILENAME`, `$CLIPBOARD`…). */
export interface VariableContext {
  selectedText: string
  currentLine: string
  currentWord: string
  /** Linha do cursor, 0-based. */
  lineIndex: number
  /** Caminho absoluto do arquivo aberto (vazio se não houver). */
  filePath: string
  clipboard: string
  workspaceFolder: string
  now: Date
}

export function emptyContext(): VariableContext {
  return {
    selectedText: '',
    currentLine: '',
    currentWord: '',
    lineIndex: 0,
    filePath: '',
    clipboard: '',
    workspaceFolder: '',
    now: new Date(),
  }
}

const DAYS = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday']
const MONTHS = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
]

const pad = (n: number, w = 2) => String(n).padStart(w, '0')
const baseName = (p: string) => p.split(/[\\/]/).pop() ?? p

function uuid(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID()
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = Math.floor(Math.random() * 16)
    return (c === 'x' ? r : (r & 0x3) | 0x8).toString(16)
  })
}

/** Valor da variável, ou `undefined` se o nome não é conhecido. Vazio conta como "sem valor". */
export function resolveVariable(name: string, ctx: VariableContext): string | undefined {
  const d = ctx.now
  const file = baseName(ctx.filePath)
  switch (name) {
    case 'TM_SELECTED_TEXT':
      return ctx.selectedText
    case 'TM_CURRENT_LINE':
      return ctx.currentLine
    case 'TM_CURRENT_WORD':
      return ctx.currentWord
    case 'TM_LINE_INDEX':
      return String(ctx.lineIndex)
    case 'TM_LINE_NUMBER':
      return String(ctx.lineIndex + 1)
    case 'TM_FILENAME':
      return file
    case 'TM_FILENAME_BASE':
      return file.replace(/\.[^.]*$/, '')
    case 'TM_DIRECTORY':
      return ctx.filePath.replace(/[\\/][^\\/]*$/, '')
    case 'TM_FILEPATH':
      return ctx.filePath
    case 'CLIPBOARD':
      return ctx.clipboard
    case 'WORKSPACE_NAME':
      return baseName(ctx.workspaceFolder)
    case 'WORKSPACE_FOLDER':
      return ctx.workspaceFolder
    case 'CURRENT_YEAR':
      return String(d.getFullYear())
    case 'CURRENT_YEAR_SHORT':
      return String(d.getFullYear()).slice(-2)
    case 'CURRENT_MONTH':
      return pad(d.getMonth() + 1)
    case 'CURRENT_MONTH_NAME':
      return MONTHS[d.getMonth()]
    case 'CURRENT_MONTH_NAME_SHORT':
      return MONTHS[d.getMonth()].slice(0, 3)
    case 'CURRENT_DATE':
      return pad(d.getDate())
    case 'CURRENT_DAY_NAME':
      return DAYS[d.getDay()]
    case 'CURRENT_DAY_NAME_SHORT':
      return DAYS[d.getDay()].slice(0, 3)
    case 'CURRENT_HOUR':
      return pad(d.getHours())
    case 'CURRENT_MINUTE':
      return pad(d.getMinutes())
    case 'CURRENT_SECOND':
      return pad(d.getSeconds())
    case 'CURRENT_SECONDS_UNIX':
      return String(Math.floor(d.getTime() / 1000))
    case 'RANDOM':
      return pad(Math.floor(Math.random() * 1_000_000), 6)
    case 'RANDOM_HEX':
      return Math.floor(Math.random() * 0x1000000)
        .toString(16)
        .padStart(6, '0')
    case 'UUID':
      return uuid()
    case 'LINE_COMMENT':
      return '%'
    case 'BLOCK_COMMENT_START':
    case 'BLOCK_COMMENT_END':
      return ''
    default:
      return undefined
  }
}

export interface StopPart {
  from: number
  to: number
  /** Presente nos espelhos com transformação. */
  transform?: Transform
}

export interface ExpandedStop {
  index: number
  /** O primeiro é o placeholder editável; os demais são espelhos. */
  parts: StopPart[]
  choices?: string[]
}

export interface Expansion {
  text: string
  /** Em ordem de navegação: 1, 2, …, depois `$0` (sempre presente, ao fim). */
  stops: ExpandedStop[]
}

/**
 * Expande o corpo em texto puro + posições dos tab stops. `indent` é prefixado a toda linha
 * depois da primeira (como o VS Code faz com a indentação da linha onde o snippet entra).
 */
export function expandBody(
  body: string | SnippetNode[],
  ctx: VariableContext,
  indent = '',
): Expansion {
  const nodes = typeof body === 'string' ? parseSnippet(body) : body
  type StopNode = SnippetNode & { t: 'stop' }

  // O primeiro `$n` com conteúdo (ou, sem nenhum, o primeiro sem transformação) é o editável.
  const primary = new Map<number, StopNode>()
  const collect = (list: SnippetNode[]) => {
    for (const n of list) {
      if (n.t === 'stop') {
        const has = n.children.length > 0 || n.choices !== undefined
        if (!n.transform && (!primary.has(n.index) || (has && !primaryHas(primary.get(n.index))))) {
          primary.set(n.index, n)
        }
        collect(n.children)
      } else if (n.t === 'var') collect(n.children)
    }
  }
  const primaryHas = (n: StopNode | undefined) =>
    !!n && (n.children.length > 0 || n.choices !== undefined)
  collect(nodes)

  const varText = (n: SnippetNode & { t: 'var' }): string => {
    const value = resolveVariable(n.name, ctx)
    if (value === undefined) return n.children.length ? plain(n.children) : n.name
    if (n.transform) return applyTransform(value, n.transform)
    return value || plain(n.children)
  }
  const plain = (list: SnippetNode[]): string => {
    let s = ''
    for (const n of list) {
      if (n.t === 'text') s += n.value
      else if (n.t === 'stop') s += plain(n.children)
      else s += varText(n)
    }
    return s
  }

  let out = ''
  const byIndex = new Map<number, ExpandedStop>()
  const emit = (value: string) => {
    out += indent ? value.replace(/\n/g, `\n${indent}`) : value
  }

  const walk = (list: SnippetNode[]) => {
    for (const n of list) {
      if (n.t === 'text') emit(n.value)
      else if (n.t === 'var') emit(varText(n))
      else {
        const stop = byIndex.get(n.index) ?? { index: n.index, parts: [] }
        byIndex.set(n.index, stop)
        const from = out.length
        if (primary.get(n.index) === n) {
          if (n.choices) stop.choices = n.choices
          walk(n.children)
          stop.parts.unshift({ from, to: out.length })
        } else {
          const prim = primary.get(n.index)
          const base = prim ? plain(prim.children) : ''
          emit(n.transform ? applyTransform(base, n.transform) : base)
          stop.parts.push({ from, to: out.length, transform: n.transform })
        }
      }
    }
  }
  walk(nodes)

  // Espelhos que vieram antes do editável não deslocam nada: posições já são finais.
  // Sem editável (só `${1/…/}`), o tab stop não é navegável.
  const stops = [...byIndex.values()]
    .filter((s) => s.index !== 0 && !s.parts[0].transform)
    .sort((a, b) => a.index - b.index)
  stops.push(byIndex.get(0) ?? { index: 0, parts: [{ from: out.length, to: out.length }] })
  return { text: out, stops }
}
