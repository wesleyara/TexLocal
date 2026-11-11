import { EditorState } from '@codemirror/state'
import { EditorView } from '@codemirror/view'
import { describe, expect, it } from 'vitest'
import { snippetPreview } from '../editor/completions'
import { applyExpansion, nextStop, prevStop, snippetActive, snippetEngine } from './engine'
import { emptyContext, expandBody } from './expand'
import {
  appliesTo,
  mergeSnippets,
  parseSnippetFile,
  searchSnippets,
  stripJsonc,
  toVsCodeJson,
  type Snippet,
} from './model'
import { applyTransform, parseSnippet } from './parser'

const ctx = { ...emptyContext(), now: new Date(2026, 10, 9, 8, 7, 6) }

describe('parser', () => {
  it('lê tab stops, placeholders, escolhas e variáveis', () => {
    const nodes = parseSnippet('a $1 ${2:x} ${3|u,v|} $TM_FILENAME ${CURRENT_YEAR} $0')
    expect(nodes.filter((n) => n.t === 'stop').map((n) => (n.t === 'stop' ? n.index : -1))).toEqual(
      [1, 2, 3, 0],
    )
    const choice = nodes.find((n) => n.t === 'stop' && n.index === 3)
    expect(choice && choice.t === 'stop' && choice.choices).toEqual(['u', 'v'])
    expect(nodes.filter((n) => n.t === 'var')).toHaveLength(2)
  })

  it('trata escapes e `$` solto como texto', () => {
    const [n] = parseSnippet('custa \\$5 \\\\ e 5$')
    expect(n).toEqual({ t: 'text', value: 'custa $5 \\ e 5$' })
  })

  it('aninha placeholders', () => {
    const [n] = parseSnippet('${1:a ${2:b}}')
    expect(n.t === 'stop' && n.children.map((c) => c.t)).toEqual(['text', 'stop'])
  })

  it('transformações regex', () => {
    const [n] = parseSnippet('${1/(\\w+)-(\\w+)/${2:/upcase}_$1/}')
    expect(n.t === 'stop' && n.transform).toBeTruthy()
    if (n.t === 'stop' && n.transform) {
      expect(applyTransform('foo-bar', n.transform)).toBe('BAR_foo')
    }
  })

  it('não lança com sintaxe quebrada', () => {
    expect(() => parseSnippet('${1:abc ${')).not.toThrow()
    expect(() => parseSnippet('${1|a,b}')).not.toThrow()
  })
})

describe('expansão', () => {
  it('posiciona tab stops e usa $0 como fim', () => {
    const ex = expandBody('\\frac{${1:a}}{${2:b}}$0', ctx)
    expect(ex.text).toBe('\\frac{a}{b}')
    expect(ex.stops.map((s) => s.index)).toEqual([1, 2, 0])
    expect(ex.stops[0].parts[0]).toEqual({ from: 6, to: 7 })
    expect(ex.stops[2].parts[0]).toEqual({ from: 11, to: 11 })
  })

  it('espelha placeholders, também antes do editável', () => {
    const ex = expandBody('\\begin{${1:env}}\\end{$1}', ctx)
    expect(ex.text).toBe('\\begin{env}\\end{env}')
    expect(ex.stops[0].parts).toHaveLength(2)
    expect(ex.stops[0].parts[0]).toEqual({ from: 7, to: 10 })
    expect(ex.stops[0].parts[1].from).toBe(16)
  })

  it('aplica transformação no espelho', () => {
    const ex = expandBody('${1:Meu Titulo} ${1/ /-/g}', ctx)
    expect(ex.text).toBe('Meu Titulo Meu-Titulo')
  })

  it('resolve variáveis', () => {
    const c = { ...ctx, filePath: 'C:\\proj\\main.tex', selectedText: 'sel', clipboard: 'clip' }
    expect(expandBody('$TM_FILENAME|$TM_FILENAME_BASE|$TM_SELECTED_TEXT|$CLIPBOARD', c).text).toBe(
      'main.tex|main|sel|clip',
    )
    expect(expandBody('$CURRENT_YEAR-$CURRENT_MONTH-$CURRENT_DATE', c).text).toBe('2026-11-09')
    expect(expandBody('${UNKNOWN_VAR}', c).text).toBe('UNKNOWN_VAR')
    expect(expandBody('${TM_SELECTED_TEXT:padrão}', ctx).text).toBe('padrão')
  })

  it('indenta as linhas seguintes', () => {
    expect(expandBody('a\nb', ctx, '  ').text).toBe('a\n  b')
  })

  it('escolha usa a primeira opção', () => {
    const ex = expandBody('${1|pmatrix,bmatrix|}', ctx)
    expect(ex.text).toBe('pmatrix')
    expect(ex.stops[0].choices).toEqual(['pmatrix', 'bmatrix'])
  })

  it('prévia mostra os valores padrão', () => {
    expect(snippetPreview('\\textbf{${1:texto}}$0')).toBe('\\textbf{texto}')
  })
})

describe('arquivos de snippets', () => {
  const meta = { scope: 'global' as const, category: 'x', file: 'x.json' }

  it('lê formato VS Code com comentários, vírgulas finais e prefix em lista', () => {
    const text = `{
      // comentário
      "Fig": { "prefix": ["fig", "figure"], "body": ["a", "b"], "scope": "latex, bibtex", "description": "d", },
    }`
    const { snippets, problems } = parseSnippetFile(text, meta)
    expect(problems).toEqual([])
    expect(snippets[0]).toMatchObject({
      name: 'Fig',
      prefixes: ['fig', 'figure'],
      body: 'a\nb',
      languages: ['latex', 'bibtex'],
    })
    expect(stripJsonc('{"a":"//não"}')).toBe('{"a":"//não"}')
  })

  it('reporta JSON inválido e definições incompletas', () => {
    expect(parseSnippetFile('{ oops', meta).problems).toHaveLength(1)
    const r = parseSnippetFile(
      '{"a":{"body":"x"},"b":{"prefix":"p"},"c":{"prefix":"c","body":"x"}}',
      meta,
    )
    expect(r.problems).toHaveLength(2)
    expect(r.snippets).toHaveLength(1)
  })

  it('exporta e reimporta sem perda', () => {
    const { snippets } = parseSnippetFile(
      '{"A":{"prefix":"a","body":["x","$0"],"scope":"latex","description":"d"}}',
      meta,
    )
    const again = parseSnippetFile(toVsCodeJson(snippets), meta).snippets
    expect(again).toEqual(snippets)
  })

  const snip = (
    scope: Snippet['scope'],
    name: string,
    prefixes: string[],
    languages: string[] = [],
  ): Snippet => ({
    id: `${scope}:${name}`,
    name,
    prefixes,
    body: name,
    description: '',
    languages,
    scope,
    category: 'c',
    file: '',
  })

  it('projeto sobrescreve global, que sobrescreve o padrão (mesmo prefixo)', () => {
    const merged = mergeSnippets([
      [snip('builtin', 'b', ['fig']), snip('builtin', 'keep', ['k'])],
      [snip('global', 'g', ['fig'])],
      [snip('project', 'p', ['fig'])],
    ])
    expect(merged.map((s) => s.name).sort()).toEqual(['keep', 'p'])
  })

  it('só o prefixo coberto é perdido em snippets com vários prefixos', () => {
    const merged = mergeSnippets([
      [snip('builtin', 'b', ['a', 'b'])],
      [snip('project', 'p', ['a'])],
    ])
    expect(merged.find((s) => s.name === 'b')?.prefixes).toEqual(['b'])
  })

  it('escopo de linguagem não conflita entre linguagens', () => {
    const merged = mergeSnippets([
      [snip('builtin', 'l', ['x'], ['latex'])],
      [snip('global', 'b', ['x'], ['bibtex'])],
    ])
    expect(merged).toHaveLength(2)
    expect(appliesTo(merged[0], 'latex')).toBe(true)
    expect(appliesTo(merged[0], 'bibtex')).toBe(false)
  })

  it('busca fuzzy por nome, prefixo ou descrição', () => {
    const list = [snip('builtin', 'Figura', ['fig']), snip('builtin', 'Tabela', ['tab'])]
    expect(searchSnippets(list, 'tab').map((s) => s.name)).toEqual(['Tabela'])
    expect(searchSnippets(list, '')).toHaveLength(2)
  })
})

function makeView(doc: string, selection?: { anchor: number; head: number }) {
  return new EditorView({
    state: EditorState.create({ doc, selection, extensions: [snippetEngine()] }),
    parent: document.body,
  })
}

describe('motor no editor', () => {
  it('navega com Tab/Shift+Tab e termina em $0', () => {
    const view = makeView('')
    applyExpansion(view, 0, 0, expandBody('[${1:a}][${2:b}]$0', ctx))
    expect(view.state.doc.toString()).toBe('[a][b]')
    expect(view.state.sliceDoc(view.state.selection.main.from, view.state.selection.main.to)).toBe(
      'a',
    )
    expect(nextStop(view)).toBe(true)
    expect(view.state.sliceDoc(view.state.selection.main.from, view.state.selection.main.to)).toBe(
      'b',
    )
    prevStop(view)
    expect(view.state.sliceDoc(view.state.selection.main.from, view.state.selection.main.to)).toBe(
      'a',
    )
    nextStop(view)
    nextStop(view)
    expect(view.state.selection.main.head).toBe(6)
    expect(snippetActive(view.state)).toBe(false)
    view.destroy()
  })

  it('espelha o que se digita no placeholder', () => {
    const view = makeView('')
    applyExpansion(view, 0, 0, expandBody('\\begin{${1:env}}\n\\end{$1}', ctx))
    const sel = view.state.selection.main
    view.dispatch({
      changes: { from: sel.from, to: sel.to, insert: 'itemize' },
      userEvent: 'input.type',
    })
    expect(view.state.doc.toString()).toBe('\\begin{itemize}\n\\end{itemize}')
    expect(snippetActive(view.state)).toBe(true)
    view.dispatch({
      changes: { from: view.state.selection.main.to, insert: 'x' },
      selection: { anchor: view.state.selection.main.to + 1 },
    })
    expect(view.state.doc.toString()).toBe('\\begin{itemizex}\n\\end{itemizex}')
    view.destroy()
  })

  it('encerra a sessão quando o cursor sai dos tab stops', () => {
    const view = makeView('fora ')
    applyExpansion(view, 5, 5, expandBody('${1:a} ${2:b}', ctx))
    expect(snippetActive(view.state)).toBe(true)
    view.dispatch({ selection: { anchor: 0 } })
    expect(snippetActive(view.state)).toBe(false)
    view.destroy()
  })
})

describe('biblioteca padrão', () => {
  const files = import.meta.glob('./builtin/*.json', {
    query: '?raw',
    import: 'default',
    eager: true,
  }) as Record<string, string>

  it('todos os arquivos são válidos e cobrem os itens do backlog', () => {
    const names: string[] = []
    for (const [path, text] of Object.entries(files)) {
      const { snippets, problems } = parseSnippetFile(text, {
        scope: 'builtin',
        category: path,
        file: '',
      })
      expect(problems, path).toEqual([])
      for (const s of snippets) {
        // O corpo precisa expandir sem lançar e produzir texto.
        expect(expandBody(s.body, ctx).text.length, s.name).toBeGreaterThan(0)
        names.push(...s.prefixes)
      }
    }
    for (const p of [
      'fig',
      'tab',
      'eq',
      'ali',
      'itm',
      'enu',
      'sec*',
      'frame',
      'thm',
      'lst',
      'minted',
      'cite',
      'mat',
      'frac',
      'SI',
    ]) {
      expect(names, p).toContain(p)
    }
  })
})
