import { CompletionContext, type CompletionResult } from '@codemirror/autocomplete'
import { StringStream, type StreamParser } from '@codemirror/language'
import { EditorState } from '@codemirror/state'
import type { EditorView } from '@codemirror/view'
import { describe, expect, it } from 'vitest'
import { envAt, envRenameSync, findMatchingEnv, insertEndOnEnter } from './behaviors'
import {
  effectivePackages,
  innermostOpenEnv,
  latexCompletionSources,
  type CompletionData,
} from './completions'
import { signatureToSnippet } from './latexData'
import { bibtexParser, languageIdFor, latexParser } from './languages'
import { labelAt } from './refs'

function tokens<S>(parser: StreamParser<S>, source: string): [string, string | null][] {
  const state = parser.startState!(2)
  const out: [string, string | null][] = []
  for (const line of source.split('\n')) {
    const stream = new StringStream(line, 2, 2)
    while (!stream.eol()) {
      const style = parser.token(stream, state)
      if (stream.pos === stream.start) throw new Error('token sem avanço')
      if (style) out.push([stream.current(), style])
      stream.start = stream.pos
    }
  }
  return out
}

describe('linguagem LaTeX', () => {
  it('reconhece comandos, ambientes, math e comentários', () => {
    const t = tokens(latexParser, '\\begin{align} x_1 \\end{align} $a$ % nota')
    expect(t).toContainEqual(['\\begin', 'keyword'])
    expect(t).toContainEqual(['align', 'tag'])
    expect(t).toContainEqual(['x', 'atom'])
    expect(t).toContainEqual(['% nota', 'comment'])
  })

  it('trata verbatim como literal até o \\end', () => {
    const t = tokens(latexParser, '\\begin{verbatim}\n\\nao{comando}\n\\end{verbatim} \\sim')
    expect(t.find(([s]) => s.includes('nao'))?.[1]).toBe('string')
    expect(t).toContainEqual(['\\sim', 'keyword'])
  })

  it('math inline termina no segundo cifrão', () => {
    const t = tokens(latexParser, '$a$ texto')
    expect(t.find(([s]) => s === 'texto')).toBeUndefined() // texto comum não recebe estilo
    expect(t.filter(([, s]) => s === 'atom')).toHaveLength(1)
  })

  it('escolhe a linguagem pela extensão', () => {
    expect(languageIdFor('a/b/main.tex')).toBe('latex')
    expect(languageIdFor('refs.bib')).toBe('bibtex')
    expect(languageIdFor('LEIAME.md')).toBe('markdown')
    expect(languageIdFor('notas.txt')).toBe('text')
  })
})

describe('linguagem BibTeX', () => {
  it('colore tipo, chave, campo e valor', () => {
    const t = tokens(bibtexParser, '@book{knuth84,\n  title = {The TeXbook},\n  year = 1984\n}')
    expect(t).toContainEqual(['@book', 'keyword'])
    expect(t).toContainEqual(['knuth84', 'atom'])
    expect(t).toContainEqual(['title', 'tag'])
    expect(t.some(([s, k]) => s.includes('TeXbook') && k === 'string')).toBe(true)
    expect(t).toContainEqual(['1984', 'number'])
  })
})

describe('\\begin / \\end', () => {
  const doc = '\\begin{a}\n\\begin{b}\nx\n\\end{b}\n\\end{a}\n'

  it('encontra o par respeitando o aninhamento', () => {
    const state = EditorState.create({ doc })
    const begin = envAt(state.doc, 8)!
    expect(begin.name).toBe('a')
    const end = findMatchingEnv(state.doc, begin)!
    expect(state.sliceDoc(end.from - 5, end.to)).toBe('\\end{a')
    expect(findMatchingEnv(state.doc, end)?.from).toBe(begin.from)
  })

  it('renomear o \\begin renomeia o \\end', () => {
    let state = EditorState.create({ doc, extensions: [envRenameSync] })
    const begin = envAt(state.doc, 8)!
    state = state.update({ changes: { from: begin.from, to: begin.to, insert: 'novo' } }).state
    expect(state.doc.toString()).toBe('\\begin{novo}\n\\begin{b}\nx\n\\end{b}\n\\end{novo}\n')
  })

  it('renomear o \\end renomeia o \\begin', () => {
    let state = EditorState.create({ doc, extensions: [envRenameSync] })
    const end = envAt(state.doc, doc.indexOf('\\end{b}') + 6)!
    state = state.update({ changes: { from: end.from, to: end.to, insert: 'c' } }).state
    expect(state.doc.toString()).toBe('\\begin{a}\n\\begin{c}\nx\n\\end{c}\n\\end{a}\n')
  })

  it('desfazer reverte begin e end juntos', () => {
    let state = EditorState.create({ doc, extensions: [envRenameSync] })
    const begin = envAt(state.doc, 8)!
    const tr = state.update({ changes: { from: begin.from, to: begin.to, insert: 'novo' } })
    state = tr.state
    expect(tr.changes.invert(EditorState.create({ doc }).doc).apply(state.doc).toString()).toBe(doc)
  })

  function press(doc: string, at: number) {
    let state = EditorState.create({ doc, selection: { anchor: at } })
    const view = {
      state,
      dispatch: (spec: Parameters<EditorState['update']>[0]) => {
        state = state.update(spec).state
        ;(view as { state: EditorState }).state = state
      },
    }
    const handled = insertEndOnEnter(view as unknown as EditorView)
    return { handled, text: state.doc.toString(), cursor: state.selection.main.head }
  }

  it('Enter após \\begin{x} sem fim cria o \\end{x}', () => {
    const src = '  \\begin{itemize}'
    const r = press(src, src.length)
    expect(r.handled).toBe(true)
    expect(r.text).toBe('  \\begin{itemize}\n    \n  \\end{itemize}')
    expect(r.cursor).toBe('  \\begin{itemize}\n    '.length)
  })

  it('não duplica quando o \\end já existe', () => {
    const src = '\\begin{itemize}\n\\end{itemize}'
    expect(press(src, '\\begin{itemize}'.length).handled).toBe(false)
  })
})

describe('autocomplete', () => {
  it('acha o ambiente aberto mais interno', () => {
    expect(innermostOpenEnv('\\begin{a}\\begin{b}x\\end{b}')).toBe('a')
    expect(innermostOpenEnv('\\begin{a}\\begin{b}')).toBe('b')
    expect(innermostOpenEnv('% \\begin{a}\n')).toBeNull()
  })

  it('gera snippets com campos a partir da assinatura', () => {
    expect(signatureToSnippet('frac', '{numerador}{denominador}')).toBe(
      '\\frac{${1:numerador}}{${2:denominador}}',
    )
    expect(signatureToSnippet('maketitle', '')).toBe('\\maketitle')
  })

  it('pacotes efetivos incluem os implícitos pela classe', () => {
    expect(effectivePackages(null, 'article')).toBeNull()
    const eff = effectivePackages(new Set(['amsmath']), 'abntex2')!
    expect(eff.has('amsmath') && eff.has('abntex2') && eff.has('')).toBe(true)
    expect(eff.has('siunitx')).toBe(false)
  })

  const data = (packages: string[] | null): CompletionData => ({
    packages: () => (packages ? new Set(packages) : null),
    documentClass: () => 'article',
    userCommands: () => [{ name: 'R', kind: 'command', args: 0, line: 0 }],
    labels: () => [
      {
        name: 'fig:g',
        span: { line: 1, col: 0, endCol: 5 },
        context: 'Um gráfico',
        file: '/p/a.tex',
      },
    ],
    files: () => ['cap1.tex', 'img/foto.png'],
  })

  async function complete(source: string, d: CompletionData): Promise<CompletionResult | null> {
    const state = EditorState.create({ doc: source })
    const ctx = new CompletionContext(state, source.length, true)
    for (const s of latexCompletionSources(d)) {
      const r = await s(ctx)
      if (r) return r
    }
    return null
  }
  const labels = (r: CompletionResult | null) => r?.options.map((o) => o.label) ?? []

  it('filtra comandos pelos pacotes carregados', async () => {
    const sem = labels(await complete('\\eqr', data([])))
    const com = labels(await complete('\\eqr', data(['amsmath'])))
    expect(sem).not.toContain('\\eqref')
    expect(com).toContain('\\eqref')
  })

  it('sem projeto indexado não filtra', async () => {
    expect(labels(await complete('\\eqr', data(null)))).toContain('\\eqref')
  })

  it('indexa comandos definidos pelo usuário', async () => {
    expect(labels(await complete('\\R', data([])))).toContain('\\R')
  })

  it('completa labels em \\ref com prévia do contexto', async () => {
    const r = await complete('\\ref{', data([]))
    expect(r?.options[0]).toMatchObject({ label: 'fig:g', detail: 'Um gráfico' })
  })

  it('completa caminhos por tipo de comando', async () => {
    expect(labels(await complete('\\includegraphics{', data([])))).toEqual(['img/foto.png'])
    expect(labels(await complete('\\input{', data([])))).toEqual(['cap1'])
  })

  it('sugere o ambiente aberto em \\end{', async () => {
    const r = await complete('\\begin{itemize}\n\\end{', data([]))
    expect(r?.options.find((o) => o.label === 'itemize')?.boost).toBe(99)
  })
})

describe('labels sob o cursor', () => {
  const state = EditorState.create({ doc: 'ver \\cref{a, bb} e \\label{x}' })

  it('identifica o item de uma lista \\cref', () => {
    expect(labelAt(state, 14)).toMatchObject({ name: 'bb', kind: 'ref' })
    expect(labelAt(state, 11)).toMatchObject({ name: 'a', kind: 'ref' })
  })

  it('identifica \\label', () => {
    const pos = state.doc.toString().indexOf('x}')
    expect(labelAt(state, pos)).toMatchObject({ name: 'x', kind: 'label' })
  })

  it('fora de um argumento devolve nulo', () => {
    expect(labelAt(state, 1)).toBeNull()
  })
})
