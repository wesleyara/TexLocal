import { invoke } from '@tauri-apps/api/core'
import { isTauri } from '../tauri'

export interface SnippetSourceFile {
  path: string
  text: string
}

export interface SnippetSources {
  globalDir: string
  projectDir: string | null
  global: SnippetSourceFile[]
  project: SnippetSourceFile[]
}

export const emptySources = (): SnippetSources => ({
  globalDir: '',
  projectDir: null,
  global: [],
  project: [],
})

export async function snippetsLoad(root: string | null): Promise<SnippetSources> {
  if (!isTauri()) return emptySources()
  return invoke<SnippetSources>('snippets_load', { root })
}

export async function snippetsWatch(root: string | null): Promise<void> {
  if (isTauri()) await invoke('snippets_watch', { root })
}

/** Chama `cb` quando um arquivo de snippets muda no disco; devolve a função que cancela. */
export async function onSnippetsChanged(cb: () => void): Promise<() => void> {
  if (!isTauri()) return () => {}
  const { listen } = await import('@tauri-apps/api/event')
  return listen('snippets-changed', cb)
}

export async function pickSnippetFiles(): Promise<string[]> {
  if (!isTauri()) return []
  const { open } = await import('@tauri-apps/plugin-dialog')
  const picked = await open({
    multiple: true,
    filters: [{ name: 'Snippets', extensions: ['json', 'code-snippets'] }],
  })
  if (!picked) return []
  return Array.isArray(picked) ? picked : [picked]
}

export async function pickSaveSnippetFile(defaultName: string): Promise<string | null> {
  if (!isTauri()) return null
  const { save } = await import('@tauri-apps/plugin-dialog')
  return save({
    defaultPath: defaultName,
    filters: [{ name: 'Snippets', extensions: ['json', 'code-snippets'] }],
  })
}
