import {
  snippet,
  snippetCompletion,
  type Completion,
  type CompletionContext,
  type CompletionResult,
  type CompletionSource,
} from '@codemirror/autocomplete'
import type { EditorView } from '@codemirror/view'
import type { UserCommand } from '../project/api'
import type { LabelDef } from '../project/model'
import {
  CLASS_PACKAGES,
  COMMANDS,
  ENVIRONMENTS,
  PACKAGE_NAMES,
  signatureToSnippet,
} from './latexData'

/** Dados do projeto que o autocomplete consulta a cada ativação. */
export interface CompletionData {
  /** `null` quando o arquivo não pertence a um projeto indexado: não filtra por pacote. */
  packages(): Set<string> | null
  documentClass(): string | null
  userCommands(): UserCommand[]
  labels(): LabelDef[]
  /** Arquivos do projeto relativos à pasta do arquivo principal, com `/`. */
  files(): string[]
}

const IMAGE_EXT = /\.(png|jpe?g|pdf|eps|svg)$/i

/** Pacotes efetivamente disponíveis (carregados + implícitos pela classe). */
export function effectivePackages(
  loaded: Set<string> | null,
  documentClass: string | null,
): Set<string> | null {
  if (!loaded) return null
  const all = new Set(loaded)
  all.add('')
  for (const p of (documentClass && CLASS_PACKAGES[documentClass]) || []) all.add(p)
  if (documentClass) all.add(documentClass)
  return all
}

/** Ambiente aberto mais interno e ainda não fechado em `before`. */
export function innermostOpenEnv(before: string): string | null {
  const stack: string[] = []
  const re = /\\(begin|end)\{([^}]*)\}/g
  const clean = before.replace(/(^|[^\\])%.*$/gm, '$1')
  for (let m = re.exec(clean); m; m = re.exec(clean)) {
    if (m[1] === 'begin') stack.push(m[2])
    else {
      const i = stack.lastIndexOf(m[2])
      if (i >= 0) stack.length = i
    }
  }
  return stack.at(-1) ?? null
}

function commandSource(data: CompletionData): CompletionSource {
  return (context) => {
    const word = context.matchBefore(/\\[A-Za-z@]*$/)
    if (!word) return null
    return build(word.from)

    function build(from: number): CompletionResult {
      const allowed = effectivePackages(data.packages(), data.documentClass())
      const options: Completion[] = []
      for (const c of COMMANDS) {
        if (allowed && !allowed.has(c.pkg)) continue
        const template = signatureToSnippet(c.name, c.sig)
        const base = {
          label: `\\${c.name}`,
          detail: c.sig,
          info: c.pkg ? `${c.doc} (${c.pkg})` : c.doc,
          type: 'function',
        }
        options.push(c.sig ? snippetCompletion(template, base) : { ...base, apply: `\\${c.name}` })
      }
      for (const u of data.userCommands()) {
        if (u.kind === 'environment') continue
        const sig = '{arg}'.repeat(u.args)
        options.push(
          snippetCompletion(signatureToSnippet(u.name, sig), {
            label: `\\${u.name}`,
            detail: sig,
            info: u.kind === 'operator' ? 'Operador do projeto' : 'Comando do projeto',
            type: 'function',
            boost: 5,
          }),
        )
      }
      return { from, options, validFor: /^\\[A-Za-z@]*$/ }
    }
  }
}

function envSource(data: CompletionData): CompletionSource {
  return (context) => {
    const m = context.matchBefore(/\\(?:begin|end)\{[A-Za-z*]*$/)
    if (!m) return null
    const isBegin = m.text.startsWith('\\begin')
    const from = m.from + m.text.indexOf('{') + 1
    const allowed = effectivePackages(data.packages(), data.documentClass())
    const names = new Map<string, { doc: string; args?: string; boost: number }>()
    for (const e of ENVIRONMENTS) {
      if (allowed && !allowed.has(e.pkg)) continue
      names.set(e.name, { doc: e.doc, args: e.args, boost: 0 })
    }
    for (const u of data.userCommands()) {
      if (u.kind === 'environment') names.set(u.name, { doc: 'Ambiente do projeto', boost: 5 })
    }
    if (!isBegin) {
      const open = innermostOpenEnv(context.state.sliceDoc(0, m.from))
      if (open) names.set(open, { doc: 'Fecha o ambiente aberto', boost: 99 })
    }
    const options: Completion[] = [...names].map(([name, info]) => ({
      label: name,
      detail: info.args,
      info: info.doc,
      type: 'class',
      boost: info.boost,
      apply: (view: EditorView, completion: Completion, f: number, to: number) => {
        // closeBrackets já pode ter inserido o `}`: consome-o para não duplicar.
        const end = view.state.sliceDoc(to, to + 1) === '}' ? to + 1 : to
        if (isBegin) {
          const indent = /^\s*/.exec(view.state.doc.lineAt(f).text)?.[0] ?? ''
          const args = info.args ? info.args.replace(/([[{])([^\]}]*)([\]}])/g, '$1${$2}$3') : ''
          const template = `${name}}${args}\n${indent}  \${}\n${indent}\\end{${name}}`
          snippet(template)(view, completion, f, end)
        } else {
          view.dispatch({
            changes: { from: f, to: end, insert: `${name}}` },
            selection: { anchor: f + name.length + 1 },
            userEvent: 'input.complete',
          })
        }
      },
    }))
    return { from, options, validFor: /^[A-Za-z*]*$/ }
  }
}

function refSource(data: CompletionData): CompletionSource {
  return (context) => {
    const m = context.matchBefore(
      /\\(?:eqref|ref|autoref|cref|Cref|pageref|nameref|vref|subref)\*?(?:\[[^\]]*\])?\{[^}]*$/,
    )
    if (!m) return null
    const from = m.from + Math.max(m.text.lastIndexOf('{'), m.text.lastIndexOf(',')) + 1
    const options: Completion[] = data.labels().map((l) => ({
      label: l.name,
      detail: l.context,
      info: `${l.file.split(/[\\/]/).pop()}:${l.span.line + 1}`,
      type: 'constant',
    }))
    return { from, options, validFor: /^[^,}]*$/ }
  }
}

function packageSource(): CompletionSource {
  return (context) => {
    const m = context.matchBefore(/\\(?:usepackage|RequirePackage)(?:\[[^\]]*\])?\{[^}]*$/)
    if (!m) return null
    const from = m.from + Math.max(m.text.lastIndexOf('{'), m.text.lastIndexOf(',')) + 1
    return {
      from,
      options: PACKAGE_NAMES.map((label) => ({ label, type: 'namespace' })),
      validFor: /^[^,}]*$/,
    }
  }
}

function pathSource(data: CompletionData): CompletionSource {
  return (context: CompletionContext) => {
    const m = context.matchBefore(
      /\\(includegraphics|input|include|subfile)\*?(?:\[[^\]]*\])?\{[^}]*$/,
    )
    if (!m) return null
    const command = /^\\(\w+)/.exec(m.text)?.[1] ?? ''
    const from = m.from + m.text.lastIndexOf('{') + 1
    const graphics = command === 'includegraphics'
    const options: Completion[] = data
      .files()
      .filter((f) => (graphics ? IMAGE_EXT.test(f) : f.endsWith('.tex')))
      .map((f) => ({
        label: graphics ? f : f.replace(/\.tex$/, ''),
        type: 'text',
      }))
    return { from, options, validFor: /^[^}]*$/ }
  }
}

export function latexCompletionSources(data: CompletionData): CompletionSource[] {
  return [envSource(data), refSource(data), packageSource(), pathSource(data), commandSource(data)]
}
