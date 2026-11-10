import { describe, expect, it } from 'vitest'
import type { FileIndex, ProjectView } from './api'
import {
  applyEdits,
  buildOutline,
  currentOutlineIndex,
  isValidLabelName,
  labelProblems,
  loadedPackages,
  renameLabelEdits,
} from './model'
import { buildTree, flattenTree } from './tree'

const empty = (): FileIndex => ({
  documentClass: null,
  magicRoot: null,
  sections: [],
  labels: [],
  references: [],
  citations: [],
  includes: [],
  packages: [],
  commands: [],
  floats: [],
})

const span = (line: number, col: number, endCol: number) => ({ line, col, endCol })

function view(): ProjectView {
  const main = empty()
  main.documentClass = 'article'
  main.packages = [{ name: 'amsmath', options: '' }]
  main.sections = [
    { kind: 'section', level: 1, title: 'Intro', starred: false, line: 2 },
    { kind: 'subsection', level: 2, title: 'Detalhe', starred: false, line: 10 },
  ]
  main.labels = [{ name: 'sec:intro', span: span(2, 30, 39), context: 'Intro' }]
  main.references = [
    { name: 'sec:intro', command: 'ref', span: span(4, 6, 15) },
    { name: 'fig:nao', command: 'cref', span: span(5, 7, 14) },
  ]
  const cap = empty()
  cap.sections = [{ kind: 'section', level: 1, title: 'Capítulo', starred: false, line: 0 }]
  cap.floats = [{ kind: 'figure', caption: 'Um gráfico', label: 'fig:g', line: 3 }]
  cap.labels = [{ name: 'sec:intro', span: span(1, 7, 16), context: '' }]
  cap.references = [{ name: 'sec:intro', command: 'eqref', span: span(2, 0, 9) }]
  return {
    main: '/p/main.tex',
    files: [
      { path: '/p/main.tex', index: main },
      { path: '/p/cap.tex', index: cap },
    ],
    missing: [],
  }
}

describe('outline', () => {
  it('atravessa os arquivos na ordem do documento e inclui figuras', () => {
    const items = buildOutline(view())
    expect(items.map((i) => [i.kind, i.title])).toEqual([
      ['section', 'Intro'],
      ['section', 'Detalhe'],
      ['section', 'Capítulo'],
      ['figure', 'Um gráfico'],
    ])
  })

  it('destaca a seção do cursor', () => {
    const items = buildOutline(view())
    expect(currentOutlineIndex(items, '/p/main.tex', 1)).toBe(-1)
    expect(currentOutlineIndex(items, '/p/main.tex', 5)).toBe(0)
    expect(currentOutlineIndex(items, '/p/main.tex', 12)).toBe(1)
    expect(currentOutlineIndex(items, '\\p\\cap.tex', 3)).toBe(2)
  })
})

describe('referências', () => {
  it('acusa label duplicado e referência inexistente', () => {
    const v = view()
    const main = labelProblems(v, '/p/main.tex')
    expect(main.map((p) => [p.messageKey, p.params.name])).toEqual([
      ['editor.problem.duplicateLabel', 'sec:intro'],
      ['editor.problem.undefinedRef', 'fig:nao'],
    ])
  })

  it('não afirma referência inexistente se há arquivos faltando', () => {
    const v = { ...view(), missing: ['x'] }
    expect(labelProblems(v, '/p/main.tex').every((p) => p.severity === 'warning')).toBe(true)
  })

  it('renomear label atualiza definição e usos em todos os arquivos', () => {
    const edits = renameLabelEdits(view(), 'sec:intro', 'sec:inicio')
    expect([...edits.keys()].sort()).toEqual(['/p/cap.tex', '/p/main.tex'])
    expect(edits.get('/p/main.tex')).toHaveLength(2)
    expect(edits.get('/p/cap.tex')).toHaveLength(2)
  })

  it('aplica edições de trás para a frente na mesma linha', () => {
    const text = 'a \\ref{x} \\ref{x}\nb'
    const out = applyEdits(text, [
      { line: 0, col: 7, endCol: 8, insert: 'novo' },
      { line: 0, col: 15, endCol: 16, insert: 'novo' },
    ])
    expect(out).toBe('a \\ref{novo} \\ref{novo}\nb')
  })

  it('valida nomes de label', () => {
    expect(isValidLabelName('sec:intro-1')).toBe(true)
    expect(isValidLabelName('com espaço')).toBe(false)
    expect(isValidLabelName('a,b')).toBe(false)
    expect(isValidLabelName('')).toBe(false)
  })
})

describe('contexto do projeto', () => {
  it('coleta os pacotes carregados', () => {
    expect([...loadedPackages(view())]).toEqual(['amsmath'])
  })
})

describe('árvore de arquivos', () => {
  const paths = ['main.tex', 'chapters/b.tex', 'chapters/a.tex', 'img/f.png']

  it('agrupa por pasta, pastas antes de arquivos, em ordem alfabética', () => {
    const rows = flattenTree(buildTree(paths), new Set())
    expect(rows.map((r) => `${' '.repeat(r.depth)}${r.name}`)).toEqual([
      'chapters',
      ' a.tex',
      ' b.tex',
      'img',
      ' f.png',
      'main.tex',
    ])
  })

  it('esconde o conteúdo de pastas recolhidas', () => {
    const rows = flattenTree(buildTree(paths), new Set(['chapters']))
    expect(rows.map((r) => r.name)).toEqual(['chapters', 'img', 'f.png', 'main.tex'])
  })
})
