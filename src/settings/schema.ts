export type ThemeSetting = 'system' | 'dark' | 'light'
export type LocaleSetting = 'auto' | 'pt-BR' | 'en'
export type TexSource = 'unset' | 'system' | 'managed' | 'skipped'
/** Pacote faltante na compilação: perguntar, instalar sozinho ou nunca. */
export type AutoInstall = 'ask' | 'auto' | 'never'
export type SnippetOrder = 'before' | 'after'

export interface Settings {
  theme: ThemeSetting
  locale: LocaleSetting
  /** Overrides de atalhos: id do comando -> binding. */
  keybindings: Record<string, string>
  /** Origem do TeX; 'unset' abre o assistente de primeira execução. */
  texSource: TexSource
  /** Diretório bin do TeX do sistema escolhido. */
  texBinDir?: string
  /** Espelho CTAN fixado (vazio = automático). */
  texMirror?: string
  texAutoInstall: AutoInstall
  /** Zoom da interface (cada passo = 20%); 0 = 100%. */
  zoomLevel: number
  /** Categorias da biblioteca padrão de snippets desativadas. */
  snippetsDisabled: string[]
  /** Snippets antes (`before`) ou depois (`after`) dos comandos no autocomplete. */
  snippetsOrder: SnippetOrder
}

export const ZOOM_MIN = -4
export const ZOOM_MAX = 8

export const DEFAULT_SETTINGS: Settings = {
  theme: 'dark',
  locale: 'auto',
  keybindings: {},
  texSource: 'unset',
  texAutoInstall: 'ask',
  zoomLevel: 0,
  snippetsDisabled: [],
  snippetsOrder: 'before',
}

const THEMES: ThemeSetting[] = ['system', 'dark', 'light']
const LOCALES: LocaleSetting[] = ['auto', 'pt-BR', 'en']
const TEX_SOURCES: TexSource[] = ['unset', 'system', 'managed', 'skipped']
const AUTO_INSTALLS: AutoInstall[] = ['ask', 'auto', 'never']

/** Valida um valor desconhecido (ex.: JSON editado à mão). Lança Error com mensagem legível. */
export function validateSettings(value: unknown): Settings {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error('settings deve ser um objeto')
  }
  const v = value as Record<string, unknown>
  const known = new Set([...Object.keys(DEFAULT_SETTINGS), 'texBinDir', 'texMirror'])
  for (const k of Object.keys(v)) {
    if (!known.has(k)) throw new Error(`chave desconhecida: ${k}`)
  }
  const theme = v.theme ?? DEFAULT_SETTINGS.theme
  if (!THEMES.includes(theme as ThemeSetting)) throw new Error(`theme inválido: ${String(theme)}`)
  const locale = v.locale ?? DEFAULT_SETTINGS.locale
  if (!LOCALES.includes(locale as LocaleSetting)) {
    throw new Error(`locale inválido: ${String(locale)}`)
  }
  const texSource = v.texSource ?? DEFAULT_SETTINGS.texSource
  if (!TEX_SOURCES.includes(texSource as TexSource)) {
    throw new Error(`texSource inválido: ${String(texSource)}`)
  }
  const texAutoInstall = v.texAutoInstall ?? DEFAULT_SETTINGS.texAutoInstall
  if (!AUTO_INSTALLS.includes(texAutoInstall as AutoInstall)) {
    throw new Error(`texAutoInstall inválido: ${String(texAutoInstall)}`)
  }
  const zoomLevel = v.zoomLevel ?? DEFAULT_SETTINGS.zoomLevel
  if (
    typeof zoomLevel !== 'number' ||
    !Number.isInteger(zoomLevel) ||
    zoomLevel < ZOOM_MIN ||
    zoomLevel > ZOOM_MAX
  ) {
    throw new Error(`zoomLevel inválido: ${String(zoomLevel)}`)
  }
  const snippetsDisabled = v.snippetsDisabled ?? DEFAULT_SETTINGS.snippetsDisabled
  if (!Array.isArray(snippetsDisabled) || snippetsDisabled.some((x) => typeof x !== 'string')) {
    throw new Error('snippetsDisabled deve ser uma lista de textos')
  }
  const snippetsOrder = v.snippetsOrder ?? DEFAULT_SETTINGS.snippetsOrder
  if (snippetsOrder !== 'before' && snippetsOrder !== 'after') {
    throw new Error(`snippetsOrder inválido: ${String(snippetsOrder)}`)
  }
  if (v.texBinDir !== undefined && typeof v.texBinDir !== 'string') {
    throw new Error('texBinDir deve ser texto')
  }
  if (v.texMirror !== undefined && typeof v.texMirror !== 'string') {
    throw new Error('texMirror deve ser texto')
  }
  const kb = v.keybindings ?? {}
  if (typeof kb !== 'object' || kb === null || Array.isArray(kb)) {
    throw new Error('keybindings deve ser um objeto')
  }
  for (const [id, b] of Object.entries(kb)) {
    if (typeof b !== 'string') throw new Error(`keybindings.${id} deve ser texto`)
  }
  return {
    theme: theme as ThemeSetting,
    locale: locale as LocaleSetting,
    keybindings: { ...(kb as Record<string, string>) },
    texSource: texSource as TexSource,
    texAutoInstall: texAutoInstall as AutoInstall,
    zoomLevel,
    snippetsDisabled: [...(snippetsDisabled as string[])],
    snippetsOrder,
    ...(v.texBinDir !== undefined ? { texBinDir: v.texBinDir as string } : {}),
    ...(v.texMirror !== undefined ? { texMirror: v.texMirror as string } : {}),
  }
}

/** Precedência: projeto > global > padrão. */
export function resolveSettings(
  defaults: Settings,
  global: Partial<Settings>,
  project: Partial<Settings>,
): Settings {
  return {
    ...defaults,
    ...global,
    ...project,
    keybindings: {
      ...defaults.keybindings,
      ...global.keybindings,
      ...project.keybindings,
    },
  }
}
