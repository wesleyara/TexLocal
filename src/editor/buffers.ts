/**
 * Estado dos buffers fora da reatividade do Vue: `EditorState` (undo, seleção, dobras) por arquivo.
 * Os objetos são grandes e imutáveis; só a lista de abas é reativa.
 */
import type { EditorState, Text } from '@codemirror/state'
import { pathKey } from '../project/model'

const states = new Map<string, EditorState>()
/** Texto lido do disco para arquivos cujo `EditorState` ainda não foi criado. */
const pending = new Map<string, string>()
/** Documento no momento do último salvamento/leitura (para saber se está "sujo"). */
const saved = new Map<string, Text | string>()

export const buffers = {
  setPending(path: string, text: string) {
    pending.set(pathKey(path), text)
    saved.set(pathKey(path), text)
  },
  takePending(path: string): string | undefined {
    const k = pathKey(path)
    const text = pending.get(k)
    pending.delete(k)
    return text
  },
  hasState: (path: string) => states.has(pathKey(path)),
  getState: (path: string) => states.get(pathKey(path)),
  setState(path: string, state: EditorState) {
    states.set(pathKey(path), state)
  },
  /** Texto atual do buffer (estado vivo, ou o lido do disco se ainda não aberto no editor). */
  text(path: string): string | undefined {
    const k = pathKey(path)
    return states.get(k)?.doc.toString() ?? pending.get(k)
  },
  markSaved(path: string) {
    const k = pathKey(path)
    const st = states.get(k)
    saved.set(k, st ? st.doc : (pending.get(k) ?? ''))
  },
  isDirty(path: string): boolean {
    const k = pathKey(path)
    const st = states.get(k)
    if (!st) return false
    const s = saved.get(k)
    if (s === undefined) return false
    return typeof s === 'string' ? st.doc.toString() !== s : !st.doc.eq(s)
  },
  drop(path: string) {
    const k = pathKey(path)
    states.delete(k)
    pending.delete(k)
    saved.delete(k)
  },
  /** Substitui o conteúdo (ex.: depois de renomear label em arquivo aberto sem passar pelo editor). */
  reset() {
    states.clear()
    pending.clear()
    saved.clear()
  },
}
