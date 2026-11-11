import { defineStore } from 'pinia'
import { computed, ref, watch as vueWatch } from 'vue'
import { getActiveView } from '../editor/actions'
import { registerCommand, unregisterCommand } from '../commands/registry'
import { fileRead, fileWrite, joinPath } from '../project/api'
import {
  onSnippetsChanged,
  pickSaveSnippetFile,
  pickSnippetFiles,
  snippetsLoad,
  snippetsWatch,
  type SnippetSources,
} from '../snippets/api'
import { insertSnippet } from '../snippets/engine'
import {
  mergeSnippets,
  parseSnippetFile,
  snippetToRaw,
  stripJsonc,
  toVsCodeJson,
  type Snippet,
  type SnippetProblem,
  type SnippetScope,
} from '../snippets/model'
import { useEditorStore } from './editor'
import { useNotificationsStore } from './notifications'
import { useProjectStore } from './project'
import { useSettingsStore } from './settings'

/** Biblioteca padrão: um arquivo por categoria em `src/snippets/builtin`. */
const BUILTIN_FILES = import.meta.glob('../snippets/builtin/*.json', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>

function builtinSnippets(): { snippets: Snippet[]; categories: string[] } {
  const snippets: Snippet[] = []
  const categories: string[] = []
  for (const [path, text] of Object.entries(BUILTIN_FILES).sort(([a], [b]) => a.localeCompare(b))) {
    const category = /([^/]+)\.json$/.exec(path)?.[1] ?? path
    categories.push(category)
    snippets.push(...parseSnippetFile(text, { scope: 'builtin', category, file: '' }).snippets)
  }
  return { snippets, categories }
}

const builtin = builtinSnippets()

/** Arquivo onde snippets criados na interface são gravados. */
const USER_FILE = 'texlocal.json'

export const useSnippetsStore = defineStore('snippets', () => {
  const settings = useSettingsStore()
  const sources = ref<SnippetSources | null>(null)
  const globalSnippets = ref<Snippet[]>([])
  const projectSnippets = ref<Snippet[]>([])
  const problems = ref<SnippetProblem[]>([])
  let stopWatching: (() => void) | null = null
  let watchedRoot: string | null | undefined
  let registered: string[] = []

  const categories = builtin.categories
  const disabled = computed(() => new Set(settings.effective.snippetsDisabled))

  /** Snippets ativos, já com a precedência embutido → global → projeto aplicada. */
  const all = computed(() =>
    mergeSnippets([
      builtin.snippets.filter((s) => !disabled.value.has(s.category)),
      globalSnippets.value,
      projectSnippets.value,
    ]),
  )

  /** Todos os snippets, inclusive de categorias desativadas e sobrescritos (para o editor). */
  const everything = computed(() => [
    ...builtin.snippets,
    ...globalSnippets.value,
    ...projectSnippets.value,
  ])

  function parseAll(files: SnippetSources['global'], scope: SnippetScope) {
    const snippets: Snippet[] = []
    for (const f of files) {
      const category = /([^\\/]+?)(?:\.code-snippets|\.json)$/i.exec(f.path)?.[1] ?? f.path
      const parsed = parseSnippetFile(f.text, { scope, category, file: f.path })
      snippets.push(...parsed.snippets)
      problems.value.push(...parsed.problems)
    }
    return snippets
  }

  async function reload() {
    const project = useProjectStore()
    const loaded = await snippetsLoad(project.root)
    sources.value = loaded
    problems.value = []
    globalSnippets.value = parseAll(loaded.global, 'global')
    projectSnippets.value = parseAll(loaded.project, 'project')
    syncCommands()
  }

  /** Observa os arquivos do disco; chamado de novo quando o projeto muda. */
  async function watch() {
    const root = useProjectStore().root
    if (watchedRoot === root && stopWatching) return
    watchedRoot = root
    stopWatching?.()
    await snippetsWatch(root)
    stopWatching = await onSnippetsChanged(() => void reload())
  }

  async function init() {
    await reload()
    await watch()
  }

  // Snippets com `keybinding` viram comandos (atalho + palette).
  function syncCommands() {
    for (const id of registered) unregisterCommand(id)
    registered = []
    for (const s of all.value) {
      if (!s.keybinding) continue
      const id = `snippet.insert:${s.id}`
      registered.push(id)
      registerCommand({
        id,
        titleKey: id,
        title: s.name,
        keybinding: s.keybinding,
        run: () => void insertInActive(s),
      })
    }
  }

  vueWatch(all, syncCommands)

  async function insertInActive(s: Pick<Snippet, 'body'>) {
    const view = getActiveView()
    if (!view) return
    const editor = useEditorStore()
    await insertSnippet(view, s.body, {
      ctx: { filePath: editor.active ?? '', workspaceFolder: useProjectStore().root ?? '' },
    })
    view.focus()
  }

  function dirFor(scope: 'global' | 'project'): string | null {
    const dir = scope === 'global' ? sources.value?.globalDir : sources.value?.projectDir
    return dir || null
  }

  async function readRaw(path: string): Promise<Record<string, unknown>> {
    let text = ''
    try {
      text = (await fileRead(path)).text
    } catch {
      return {}
    }
    if (!text.trim()) return {}
    const data: unknown = JSON.parse(stripJsonc(text))
    if (typeof data !== 'object' || data === null || Array.isArray(data)) {
      throw new Error('o arquivo deve ser um objeto JSON')
    }
    return data as Record<string, unknown>
  }

  async function writeRaw(path: string, data: Record<string, unknown>) {
    await fileWrite(path, JSON.stringify(data, null, 2) + '\n', 'lf', 'utf8', false)
  }

  /**
   * Grava (cria ou substitui) um snippet no escopo `scope`. `previous` é o snippet editado:
   * se ele já vive em um arquivo desse escopo, é atualizado ali; senão vai para `texlocal.json`.
   */
  async function save(scope: 'global' | 'project', snippet: Snippet, previous?: Snippet) {
    const dir = dirFor(scope)
    if (!dir) throw new Error('escopo indisponível: abra um projeto')
    const file =
      previous && previous.scope === scope && previous.file
        ? previous.file
        : joinPath(dir, USER_FILE)
    const data = await readRaw(file)
    if (previous && previous.scope === scope && previous.name !== snippet.name)
      delete data[previous.name]
    data[snippet.name] = snippetToRaw(snippet)
    await writeRaw(file, data)
    await reload()
  }

  async function remove(snippet: Snippet) {
    if (snippet.scope === 'builtin' || !snippet.file) return
    const data = await readRaw(snippet.file)
    delete data[snippet.name]
    await writeRaw(snippet.file, data)
    await reload()
  }

  /** Importa arquivos `.json`/`.code-snippets` do VS Code (sem conversão) para `scope`. */
  async function importFiles(scope: 'global' | 'project'): Promise<number> {
    const dir = dirFor(scope)
    if (!dir) throw new Error('escopo indisponível: abra um projeto')
    const picked = await pickSnippetFiles()
    let count = 0
    for (const path of picked) {
      const { text } = await fileRead(path)
      const parsed = parseSnippetFile(text, { scope, category: 'import', file: path })
      if (parsed.problems.length && !parsed.snippets.length) {
        throw new Error(`${path}: ${parsed.problems[0].message}`)
      }
      const name = path.split(/[\\/]/).pop() ?? 'import.json'
      const target = joinPath(
        dir,
        name.endsWith('.code-snippets') ? name.replace(/\.code-snippets$/, '.json') : name,
      )
      await fileWrite(target, toVsCodeJson(parsed.snippets), 'lf', 'utf8', false)
      count += parsed.snippets.length
    }
    await reload()
    return count
  }

  /** Exporta a coleção para um arquivo escolhido pelo usuário; devolve o caminho ou `null`. */
  async function exportSnippets(list: Snippet[]): Promise<string | null> {
    const target = await pickSaveSnippetFile('snippets.code-snippets')
    if (!target) return null
    await fileWrite(target, toVsCodeJson(list), 'lf', 'utf8', false)
    return target
  }

  /** Cria o arquivo `texlocal.json` do escopo (se preciso) e o abre no editor. */
  async function openJson(scope: 'global' | 'project', snippet?: Snippet) {
    const notify = useNotificationsStore()
    const dir = dirFor(scope)
    if (!dir) {
      notify.notify('info', 'Abra um projeto para editar os snippets do projeto.')
      return
    }
    const file =
      snippet && snippet.scope === scope && snippet.file ? snippet.file : joinPath(dir, USER_FILE)
    try {
      await readRaw(file).then(async (d) => {
        if (!Object.keys(d).length) await writeRaw(file, {})
      })
    } catch {
      // arquivo com JSON inválido: abre assim mesmo para o usuário corrigir
    }
    await useEditorStore().openFile(file)
  }

  return {
    sources,
    problems,
    categories,
    all,
    everything,
    disabled,
    globalSnippets,
    projectSnippets,
    init,
    reload,
    watch,
    save,
    remove,
    importFiles,
    exportSnippets,
    openJson,
    insertInActive,
    dirFor,
  }
})
