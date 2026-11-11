import { describe, expect, it } from 'vitest'
import type { CommitInfo } from './api'
import { buildRows, diffSeq, hunksOf, revertHunk, withContext, wordSegments } from './diff'
import { dayKey, groupSessions } from './model'

describe('diffSeq', () => {
  it('encontra o menor script de edição', () => {
    const ops = diffSeq(['a', 'b', 'c', 'd'], ['a', 'c', 'd', 'e'])
    expect(ops.map((o) => `${o.kind[0]}:${o.text}`)).toEqual(['e:a', 'd:b', 'e:c', 'e:d', 'a:e'])
  })

  it('lida com vazio', () => {
    expect(diffSeq([], ['x']).map((o) => o.kind)).toEqual(['add'])
    expect(diffSeq(['x'], []).map((o) => o.kind)).toEqual(['del'])
    expect(diffSeq([], [])).toEqual([])
  })

  it('reconstrói os dois lados', () => {
    const a = 'l1 l2 l3 l4 l5 l6'.split(' ')
    const b = 'l1 x l3 l5 l6 y'.split(' ')
    const ops = diffSeq(a, b)
    expect(ops.filter((o) => o.kind !== 'add').map((o) => o.text)).toEqual(a)
    expect(ops.filter((o) => o.kind !== 'del').map((o) => o.text)).toEqual(b)
  })
})

describe('wordSegments', () => {
  it('marca só as palavras alteradas', () => {
    const s = wordSegments('o gato preto dorme', 'o gato branco dorme')
    expect(s.old.filter((x) => x.changed).map((x) => x.text)).toEqual(['preto'])
    expect(s.new.filter((x) => x.changed).map((x) => x.text)).toEqual(['branco'])
    expect(s.old.map((x) => x.text).join('')).toBe('o gato preto dorme')
  })

  it('trata acentos como parte da palavra', () => {
    const s = wordSegments('ação rápida', 'ação lenta')
    expect(s.old.find((x) => x.changed)?.text).toBe('rápida')
  })
})

describe('buildRows / hunks', () => {
  const oldText = 'a\nb\nc\nd\ne\nf\ng\nh'
  const newText = 'a\nB\nc\nd\ne\nf\ng\nh\ni'

  it('emparelha remoção+adição em change e numera as linhas', () => {
    const rows = buildRows(oldText, newText)
    const change = rows.find((r) => r.kind === 'change')
    expect(change?.old).toBe('b')
    expect(change?.new).toBe('B')
    expect(change?.oldNo).toBe(2)
    expect(rows[rows.length - 1]).toMatchObject({ kind: 'add', newNo: 9, new: 'i' })
  })

  it('reverte apenas o hunk escolhido', () => {
    const rows = buildRows(oldText, newText)
    const hunks = hunksOf(rows)
    expect(hunks).toHaveLength(2)
    expect(revertHunk(rows, hunks, 0)).toBe('a\nb\nc\nd\ne\nf\ng\nh\ni')
    expect(revertHunk(rows, hunks, 1)).toBe('a\nB\nc\nd\ne\nf\ng\nh')
  })

  it('reverter todos os hunks, um a um, converge ao texto antigo', () => {
    const rows = buildRows(oldText, newText)
    const hunks = hunksOf(rows)
    const everyHunkReverted = rows
      .map((r, i) => (hunks.some((h) => i >= h.start && i < h.end) ? r.old : r.new))
      .filter((x): x is string => x !== null)
      .join('\n')
    expect(everyHunkReverted).toBe(oldText)
  })

  it('recolhe trechos sem mudanças deixando contexto', () => {
    const many = Array.from({ length: 30 }, (_, i) => `l${i}`)
    const changed = [...many]
    changed[15] = 'X'
    const visible = withContext(buildRows(many.join('\n'), changed.join('\n')), 2)
    expect(visible.filter((v) => 'gap' in v)).toHaveLength(2)
    expect(visible.filter((v) => 'row' in v)).toHaveLength(5)
  })
})

function commit(id: string, time: number, tags: CommitInfo['tags'] = []): CommitInfo {
  return {
    id,
    short: id,
    parent: null,
    summary: id,
    message: id,
    author: 'Ana',
    email: 'a@x',
    time,
    filesChanged: 1,
    checkpoint: true,
    tags,
  }
}

describe('groupSessions', () => {
  const noon = Math.floor(new Date(2026, 0, 10, 12, 0, 0).getTime() / 1000)

  it('agrupa commits próximos e separa por lacuna longa', () => {
    const s = groupSessions([commit('a', noon), commit('b', noon - 600), commit('c', noon - 7200)])
    expect(s.map((x) => x.commits.map((c) => c.id))).toEqual([['a', 'b'], ['c']])
  })

  it('separa por dia', () => {
    const s = groupSessions([commit('a', noon), commit('b', noon - 86400)])
    expect(s).toHaveLength(2)
    expect(s[0].day).toBe(dayKey(noon))
  })

  it('mantém versões nomeadas em sessão própria', () => {
    const tag = { name: 'texlocal/v', label: 'v', description: '', target: 'b', time: noon }
    const s = groupSessions([
      commit('a', noon),
      commit('b', noon - 60, [tag]),
      commit('c', noon - 120),
    ])
    expect(s.map((x) => x.commits.length)).toEqual([1, 1, 1])
  })
})
