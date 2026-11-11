// Diff de texto para a tela de histórico: linhas (Myers), palavras dentro de linhas alteradas
// e reversão de trechos (hunks).

export type OpKind = 'equal' | 'add' | 'del'
export interface Op {
  kind: OpKind
  text: string
}

/** Acima disso o Myers desiste e trata o miolo como substituição total (arquivos gigantes). */
const MAX_D = 4000

/** Menor script de edição (Myers, O(ND)) entre duas sequências de strings. */
export function diffSeq(a: string[], b: string[]): Op[] {
  let start = 0
  while (start < a.length && start < b.length && a[start] === b[start]) start++
  let endA = a.length
  let endB = b.length
  while (endA > start && endB > start && a[endA - 1] === b[endB - 1]) {
    endA--
    endB--
  }
  const head: Op[] = a.slice(0, start).map((text) => ({ kind: 'equal', text }))
  const tail: Op[] = a.slice(endA).map((text) => ({ kind: 'equal', text }))
  return [...head, ...myers(a.slice(start, endA), b.slice(start, endB)), ...tail]
}

function myers(a: string[], b: string[]): Op[] {
  const n = a.length
  const m = b.length
  if (n === 0) return b.map((text) => ({ kind: 'add', text }))
  if (m === 0) return a.map((text) => ({ kind: 'del', text }))
  const max = Math.min(n + m, MAX_D)
  const offset = max + 1
  const v = new Int32Array(2 * max + 3)
  const trace: Int32Array[] = []
  let found = -1
  for (let d = 0; d <= max && found < 0; d++) {
    trace.push(v.slice())
    for (let k = -d; k <= d; k += 2) {
      let x: number
      if (k === -d || (k !== d && v[offset + k - 1] < v[offset + k + 1])) x = v[offset + k + 1]
      else x = v[offset + k - 1] + 1
      let y = x - k
      while (x < n && y < m && a[x] === b[y]) {
        x++
        y++
      }
      v[offset + k] = x
      if (x >= n && y >= m) {
        found = d
        break
      }
    }
  }
  if (found < 0) {
    return [
      ...a.map((text): Op => ({ kind: 'del', text })),
      ...b.map((text): Op => ({ kind: 'add', text })),
    ]
  }
  const ops: Op[] = []
  let x = n
  let y = m
  for (let d = found; d > 0; d--) {
    const prev = trace[d]
    const k = x - y
    const down = k === -d || (k !== d && prev[offset + k - 1] < prev[offset + k + 1])
    const prevK = down ? k + 1 : k - 1
    const prevX = prev[offset + prevK]
    const prevY = prevX - prevK
    while (x > prevX && y > prevY) {
      x--
      y--
      ops.push({ kind: 'equal', text: a[x] })
    }
    if (down) {
      y--
      ops.push({ kind: 'add', text: b[y] })
    } else {
      x--
      ops.push({ kind: 'del', text: a[x] })
    }
  }
  while (x > 0 && y > 0) {
    x--
    y--
    ops.push({ kind: 'equal', text: a[x] })
  }
  return ops.reverse()
}

export interface Seg {
  text: string
  changed: boolean
}

const TOKEN = /[\p{L}\p{N}_]+|\s+|[^\s\p{L}\p{N}_]/gu

/** Diff por palavra entre duas linhas (bom para prosa): segmentos de cada lado. */
export function wordSegments(oldLine: string, newLine: string): { old: Seg[]; new: Seg[] } {
  const ops = diffSeq(oldLine.match(TOKEN) ?? [], newLine.match(TOKEN) ?? [])
  const o: Seg[] = []
  const n: Seg[] = []
  const push = (list: Seg[], text: string, changed: boolean) => {
    const last = list[list.length - 1]
    if (last && last.changed === changed) last.text += text
    else list.push({ text, changed })
  }
  for (const op of ops) {
    if (op.kind !== 'add') push(o, op.text, op.kind === 'del')
    if (op.kind !== 'del') push(n, op.text, op.kind === 'add')
  }
  return { old: o, new: n }
}

export interface Row {
  kind: 'equal' | 'add' | 'del' | 'change'
  oldNo: number | null
  newNo: number | null
  old: string | null
  new: string | null
  /** Só em `change`: segmentos com as palavras alteradas. */
  oldParts?: Seg[]
  newParts?: Seg[]
}

/** Linhas do diff alinhadas; blocos de remoções+adições viram pares `change` com diff por palavra. */
export function buildRows(oldText: string, newText: string): Row[] {
  const ops = diffSeq(oldText.split('\n'), newText.split('\n'))
  const rows: Row[] = []
  let oldNo = 1
  let newNo = 1
  let i = 0
  while (i < ops.length) {
    const op = ops[i]
    if (op.kind === 'equal') {
      rows.push({ kind: 'equal', oldNo: oldNo++, newNo: newNo++, old: op.text, new: op.text })
      i++
      continue
    }
    const dels: string[] = []
    const adds: string[] = []
    while (i < ops.length && ops[i].kind !== 'equal') {
      if (ops[i].kind === 'del') dels.push(ops[i].text)
      else adds.push(ops[i].text)
      i++
    }
    const paired = Math.min(dels.length, adds.length)
    for (let j = 0; j < paired; j++) {
      const seg = wordSegments(dels[j], adds[j])
      rows.push({
        kind: 'change',
        oldNo: oldNo++,
        newNo: newNo++,
        old: dels[j],
        new: adds[j],
        oldParts: seg.old,
        newParts: seg.new,
      })
    }
    for (let j = paired; j < dels.length; j++)
      rows.push({ kind: 'del', oldNo: oldNo++, newNo: null, old: dels[j], new: null })
    for (let j = paired; j < adds.length; j++)
      rows.push({ kind: 'add', oldNo: null, newNo: newNo++, old: null, new: adds[j] })
  }
  return rows
}

export interface Hunk {
  /** Índice (em `rows`) da primeira linha alterada. */
  start: number
  /** Exclusivo. */
  end: number
}

export function hunksOf(rows: Row[]): Hunk[] {
  const hunks: Hunk[] = []
  let start = -1
  rows.forEach((r, i) => {
    if (r.kind === 'equal') {
      if (start >= 0) hunks.push({ start, end: i })
      start = -1
    } else if (start < 0) start = i
  })
  if (start >= 0) hunks.push({ start, end: rows.length })
  return hunks
}

/**
 * Texto resultante de desfazer um único hunk: o lado "novo" fica como está, exceto o trecho
 * escolhido, que volta ao conteúdo "antigo".
 */
export function revertHunk(rows: Row[], hunks: Hunk[], index: number): string {
  const target = hunks[index]
  const out: string[] = []
  rows.forEach((r, i) => {
    const inTarget = i >= target.start && i < target.end
    const line = inTarget ? r.old : r.new
    if (line !== null) out.push(line)
  })
  return out.join('\n')
}

/** Recolhe trechos longos sem mudanças, mantendo `context` linhas ao redor de cada alteração. */
export type Visible = { row: Row; index: number } | { gap: number; from: number }

export function withContext(rows: Row[], context = 3): Visible[] {
  const keep = new Array<boolean>(rows.length).fill(false)
  rows.forEach((r, i) => {
    if (r.kind === 'equal') return
    for (let j = Math.max(0, i - context); j <= Math.min(rows.length - 1, i + context); j++)
      keep[j] = true
  })
  const out: Visible[] = []
  let i = 0
  while (i < rows.length) {
    if (keep[i]) {
      out.push({ row: rows[i], index: i })
      i++
      continue
    }
    const from = i
    while (i < rows.length && !keep[i]) i++
    out.push({ gap: i - from, from })
  }
  return out
}
