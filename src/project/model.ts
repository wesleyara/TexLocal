/** Funções puras sobre o índice do projeto: outline, referências, diagnósticos e renomeação. */
import type { FileEntry, FileIndex, Label, ProjectView, Reference, Span } from './api'

/** Chave de comparação de caminhos (Windows e POSIX). */
export const pathKey = (p: string): string => p.replace(/\\/g, '/')
export const samePath = (a: string, b: string): boolean => pathKey(a) === pathKey(b)

// ───────────────────────── outline ─────────────────────────

export interface OutlineItem {
  kind: 'section' | 'figure' | 'table'
  /** Nível hierárquico: part=-1 ... subparagraph=5; figuras e tabelas ficam sob a seção. */
  level: number
  title: string
  file: string
  line: number
  starred: boolean
}

/** Sequência plana, na ordem do documento, atravessando os arquivos incluídos. */
export function buildOutline(view: ProjectView): OutlineItem[] {
  const items: OutlineItem[] = []
  for (const f of view.files) {
    const own: OutlineItem[] = [
      ...f.index.sections.map<OutlineItem>((s) => ({
        kind: 'section',
        level: s.level,
        title: s.title,
        file: f.path,
        line: s.line,
        starred: s.starred,
      })),
      ...f.index.floats.map<OutlineItem>((fl) => ({
        kind: fl.kind === 'table' ? 'table' : 'figure',
        level: 99,
        title: fl.caption || fl.label || fl.kind,
        file: f.path,
        line: fl.line,
        starred: false,
      })),
    ].sort((a, b) => a.line - b.line)
    items.push(...own)
  }
  return items
}

/** Item "atual": o último do arquivo ativo cujo início está em ou antes do cursor. */
export function currentOutlineIndex(items: OutlineItem[], file: string, line: number): number {
  let found = -1
  items.forEach((it, i) => {
    if (it.kind === 'section' && samePath(it.file, file) && it.line <= line) found = i
  })
  return found
}

// ───────────────────────── labels e referências ─────────────────────────

export interface LabelDef extends Label {
  file: string
}
export interface RefUse extends Reference {
  file: string
}

export function allLabels(view: ProjectView): LabelDef[] {
  return view.files.flatMap((f) => f.index.labels.map((l) => ({ ...l, file: f.path })))
}

export function allReferences(view: ProjectView): RefUse[] {
  return view.files.flatMap((f) => f.index.references.map((r) => ({ ...r, file: f.path })))
}

export const findLabel = (view: ProjectView, name: string): LabelDef | undefined =>
  allLabels(view).find((l) => l.name === name)

export const findUsages = (view: ProjectView, name: string): RefUse[] =>
  allReferences(view).filter((r) => r.name === name)

export interface Problem extends Span {
  severity: 'error' | 'warning'
  /** Chave de tradução + parâmetros. */
  messageKey: string
  params: Record<string, string>
}

/** Diagnósticos de label duplicado e referência inexistente para um arquivo. */
export function labelProblems(view: ProjectView, file: string): Problem[] {
  const entry = view.files.find((f) => samePath(f.path, file))
  if (!entry) return []
  const labels = allLabels(view)
  const counts = new Map<string, number>()
  for (const l of labels) counts.set(l.name, (counts.get(l.name) ?? 0) + 1)
  const defined = new Set(counts.keys())
  const problems: Problem[] = []
  for (const l of entry.index.labels) {
    if ((counts.get(l.name) ?? 0) > 1) {
      problems.push({
        ...l.span,
        severity: 'warning',
        messageKey: 'editor.problem.duplicateLabel',
        params: { name: l.name },
      })
    }
  }
  // Sem a árvore completa não dá para afirmar que um label não existe.
  if (view.missing.length === 0) {
    for (const r of entry.index.references) {
      if (!defined.has(r.name)) {
        problems.push({
          ...r.span,
          severity: 'error',
          messageKey: 'editor.problem.undefinedRef',
          params: { name: r.name },
        })
      }
    }
  }
  return problems
}

// ───────────────────────── edição ─────────────────────────

export interface TextEdit {
  line: number
  col: number
  endCol: number
  insert: string
}

/** Edições para renomear um label e todas as suas referências, por arquivo. */
export function renameLabelEdits(
  view: ProjectView,
  oldName: string,
  newName: string,
): Map<string, TextEdit[]> {
  const out = new Map<string, TextEdit[]>()
  const push = (file: string, span: Span) => {
    const list = out.get(file) ?? []
    list.push({ line: span.line, col: span.col, endCol: span.endCol, insert: newName })
    out.set(file, list)
  }
  for (const f of view.files) {
    f.index.labels.filter((l) => l.name === oldName).forEach((l) => push(f.path, l.span))
    f.index.references.filter((r) => r.name === oldName).forEach((r) => push(f.path, r.span))
  }
  return out
}

/** Aplica edições (colunas em UTF-16, linhas 0-based) a um texto. */
export function applyEdits(text: string, edits: TextEdit[]): string {
  const lines = text.split('\n')
  const ordered = [...edits].sort((a, b) => b.line - a.line || b.col - a.col)
  for (const e of ordered) {
    const line = lines[e.line]
    if (line === undefined) continue
    lines[e.line] = line.slice(0, e.col) + e.insert + line.slice(e.endCol)
  }
  return lines.join('\n')
}

/** Nome válido para label: não vazio e sem caracteres que quebram o comando. */
export const isValidLabelName = (name: string): boolean => /^[^\s{}%\\,]+$/.test(name)

// ───────────────────────── contexto do projeto ─────────────────────────

export function loadedPackages(view: ProjectView): Set<string> {
  return new Set(view.files.flatMap((f) => f.index.packages.map((p) => p.name)))
}

export function documentClass(view: ProjectView): string | null {
  return view.files.find((f) => f.index.documentClass)?.index.documentClass ?? null
}

export function userCommands(view: ProjectView): FileIndex['commands'] {
  return view.files.flatMap((f) => f.index.commands)
}

export function entryFor(view: ProjectView, file: string): FileEntry | undefined {
  return view.files.find((f) => samePath(f.path, file))
}
