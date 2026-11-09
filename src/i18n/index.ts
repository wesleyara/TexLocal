import { createI18n } from 'vue-i18n'
import ptBR from './pt-BR.json'
import en from './en.json'

export type Locale = 'pt-BR' | 'en'

export function detectLocale(language?: string): Locale {
  const lang = (
    language ?? (typeof navigator !== 'undefined' ? navigator.language : 'en')
  ).toLowerCase()
  return lang.startsWith('pt') ? 'pt-BR' : 'en'
}

export const i18n = createI18n({
  legacy: false,
  locale: detectLocale(),
  fallbackLocale: 'en',
  messages: { 'pt-BR': ptBR, en },
})

export function setLocale(setting: 'auto' | Locale): void {
  i18n.global.locale.value = setting === 'auto' ? detectLocale() : setting
  if (typeof document !== 'undefined') document.documentElement.lang = i18n.global.locale.value
}

export const t = (key: string, named?: Record<string, unknown>): string =>
  named ? i18n.global.t(key, named) : i18n.global.t(key)
