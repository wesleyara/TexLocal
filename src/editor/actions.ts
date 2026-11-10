/** Ações do editor que cruzam stores e o EditorView ativo (renomear label, navegação). */
import type { ChangeSpec } from '@codemirror/state'
import type { EditorView } from '@codemirror/view'
import { i18n } from '../i18n'
import { fileRead, fileWrite } from '../project/api'
import {
  allReferences,
  applyEdits,
  findLabel,
  isValidLabelName,
  renameLabelEdits,
  samePath,
  type TextEdit,
} from '../project/model'
import { useEditorStore } from '../stores/editor'
import { useNotificationsStore } from '../stores/notifications'
import { useProjectStore } from '../stores/project'
import { useRefsStore } from '../stores/refs'
import { buffers } from './buffers'

let activeView: EditorView | null = null
let flushIndex: (() => Promise<void>) | null = null

export function registerActiveView(view: EditorView | null, flush: (() => Promise<void>) | null) {
  activeView = view
  flushIndex = flush
}

export const getActiveView = () => activeView

const t = (key: string, params?: Record<string, unknown>) =>
  i18n.global.t(key, params ?? {}) as string

/** Vai para a definição (`\label`) de uma referência. */
export async function gotoLabel(name: string): Promise<void> {
  const label = findLabel(useProjectStore().view, name)
  if (!label) {
    useNotificationsStore().notify('error', t('editor.problem.undefinedRef', { name }))
    return
  }
  await useEditorStore().openFile(label.file, {
    line: label.span.line,
    col: label.span.col,
    endCol: label.span.endCol,
  })
}

/** Lista definição e usos de um label no painel inferior. */
export function showUsages(name: string): void {
  const project = useProjectStore()
  const refs = useRefsStore()
  const def = findLabel(project.view, name)
  refs.showUsages(
    name,
    def
      ? { file: def.file, line: def.span.line, col: def.span.col, endCol: def.span.endCol }
      : null,
    allReferences(project.view)
      .filter((r) => r.name === name)
      .map((r) => ({ file: r.file, line: r.span.line, col: r.span.col, endCol: r.span.endCol })),
  )
}

function toChanges(
  doc: { line(n: number): { from: number; length: number } },
  edits: TextEdit[],
): ChangeSpec[] {
  return edits.map((e) => {
    const line = doc.line(e.line + 1)
    return {
      from: line.from + Math.min(e.col, line.length),
      to: line.from + Math.min(e.endCol, line.length),
      insert: e.insert,
    }
  })
}

/** Renomeia o label e todas as referências, em buffers abertos e em arquivos do disco. */
export async function renameLabel(oldName: string, newName: string): Promise<boolean> {
  const notify = useNotificationsStore()
  const project = useProjectStore()
  const editor = useEditorStore()
  if (!isValidLabelName(newName)) {
    notify.notify('error', t('editor.rename.invalid'))
    return false
  }
  if (newName !== oldName && findLabel(project.view, newName)) {
    notify.notify('error', t('editor.rename.exists', { name: newName }))
    return false
  }
  await flushIndex?.()
  const edits = renameLabelEdits(project.view, oldName, newName)
  for (const [file, list] of edits) {
    const tab = editor.tabs.find((x) => samePath(x.path, file))
    const state = buffers.getState(file)
    if (tab && state) {
      const changes = toChanges(state.doc, list)
      if (activeView && editor.active && samePath(editor.active, file)) {
        activeView.dispatch({ changes, userEvent: 'input.rename' })
      } else {
        buffers.setState(file, state.update({ changes }).state)
        editor.onDocChanged(file)
        await project.updateIndex(file, buffers.text(file) ?? '')
      }
    } else {
      const content = await fileRead(file)
      const text = applyEdits(content.text, list)
      await fileWrite(file, text, content.eol, content.encoding, content.bom)
      await project.updateIndex(file, text)
    }
  }
  notify.notify('success', t('editor.rename.done', { count: edits.size }))
  return true
}
