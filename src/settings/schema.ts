export type ThemeSetting = 'system' | 'dark' | 'light'
export type LocaleSetting = 'auto' | 'pt-BR' | 'en'
export type TexSource = 'unset' | 'system' | 'managed' | 'skipped'

export interface Settings {
  theme: ThemeSetting
  locale: LocaleSetting
  /** Overrides de atalhos: id do comando -> binding. */
  keybindings: Record<string, string>
  /** Origem do TeX; 'unset' abre o assistente de primeira execução. */
  texSource: TexSource
  /** Diretório bin do TeX do sistema escolhido. */
  texBinDir?: string
}

export const DEFAULT_SETTINGS: Settings = {
  theme: 'dark',
  locale: 'auto',
  keybindings: {},
  texSource: 'unset',
}

const THEMES: ThemeSetting[] = ['system', 'dark', 'light']
const LOCALES: LocaleSetting[] = ['auto', 'pt-BR', 'en']
const TEX_SOURCES: TexSource[] = ['unset', 'system', 'managed', 'skipped']

/** Valida um valor desconhecido (ex.: JSON editado à mão). Lança Error com mensagem legível. */
export function validateSettings(value: unknown): Settings {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error('settings deve ser um objeto')
  }
  const v = value as Record<string, unknown>
  const known = new Set([...Object.keys(DEFAULT_SETTINGS), 'texBinDir'])
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
  if (v.texBinDir !== undefined && typeof v.texBinDir !== 'string') {
    throw new Error('texBinDir deve ser texto')
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
    ...(v.texBinDir !== undefined ? { texBinDir: v.texBinDir as string } : {}),
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
