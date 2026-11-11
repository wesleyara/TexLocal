export interface Command {
  id: string
  titleKey: string
  /** Título literal (comandos dinâmicos, como snippets); tem precedência sobre `titleKey`. */
  title?: string
  /** Atalho padrão, ex.: "Mod+Shift+P" (Mod = Ctrl no Windows/Linux, Cmd no macOS). */
  keybinding?: string
  run: () => void
}

export interface KeyEventLike {
  key: string
  ctrlKey: boolean
  metaKey: boolean
  shiftKey: boolean
  altKey: boolean
}

const commands = new Map<string, Command>()

export function registerCommand(command: Command): void {
  commands.set(command.id, command)
}

export function unregisterCommand(id: string): void {
  commands.delete(id)
}

/** Texto exibido do comando, traduzido por `t` quando não há título literal. */
export function commandTitle(command: Command, t: (key: string) => string): string {
  return command.title ?? t(command.titleKey)
}

export function clearCommands(): void {
  commands.clear()
}

export function allCommands(): Command[] {
  return [...commands.values()]
}

export function getCommand(id: string): Command | undefined {
  return commands.get(id)
}

export function isMac(): boolean {
  return typeof navigator !== 'undefined' && /mac/i.test(navigator.platform)
}

const MODIFIER_ORDER = ['mod', 'alt', 'shift']

/** Normaliza para forma canônica: "Mod+Shift+P" -> "mod+shift+p". */
export function normalizeBinding(binding: string): string {
  const parts = binding
    .split('+')
    .map((p) => p.trim().toLowerCase())
    .filter(Boolean)
  const key = parts.filter((p) => !MODIFIER_ORDER.includes(p))[0] ?? ''
  const mods = MODIFIER_ORDER.filter((m) => parts.includes(m))
  return [...mods, key].join('+')
}

/** Converte um evento de teclado em binding canônico (ou null se só há modificadores). */
export function eventToBinding(e: KeyEventLike, mac = isMac()): string | null {
  if (['Control', 'Meta', 'Shift', 'Alt'].includes(e.key)) return null
  const mod = mac ? e.metaKey : e.ctrlKey
  const parts: string[] = []
  if (mod) parts.push('mod')
  if (e.altKey) parts.push('alt')
  if (e.shiftKey) parts.push('shift')
  parts.push(e.key.toLowerCase())
  return parts.join('+')
}

export function matchesEvent(binding: string, e: KeyEventLike, mac = isMac()): boolean {
  return eventToBinding(e, mac) === normalizeBinding(binding)
}

/** Exibe o atalho para o SO atual: "Mod+Shift+P" -> "Ctrl+Shift+P" / "⌘⇧P". */
export function formatBinding(binding: string, mac = isMac()): string {
  const parts = normalizeBinding(binding).split('+')
  const label = (p: string) => {
    if (p === 'mod') return mac ? '⌘' : 'Ctrl'
    if (p === 'alt') return mac ? '⌥' : 'Alt'
    if (p === 'shift') return mac ? '⇧' : 'Shift'
    return p.length === 1 ? p.toUpperCase() : p.charAt(0).toUpperCase() + p.slice(1)
  }
  return mac ? parts.map(label).join('') : parts.map(label).join('+')
}

export function effectiveBinding(
  command: Command,
  overrides: Record<string, string> = {},
): string | undefined {
  return overrides[command.id] ?? command.keybinding
}

/** Retorna grupos de ids de comandos que compartilham o mesmo atalho efetivo. */
export function findConflicts(list: Command[], overrides: Record<string, string> = {}): string[][] {
  const byBinding = new Map<string, string[]>()
  for (const c of list) {
    const b = effectiveBinding(c, overrides)
    if (!b) continue
    const key = normalizeBinding(b)
    byBinding.set(key, [...(byBinding.get(key) ?? []), c.id])
  }
  return [...byBinding.values()].filter((ids) => ids.length > 1)
}

/** Pontuação fuzzy (maior = melhor); null quando não casa. */
export function fuzzyScore(query: string, text: string): number | null {
  const q = query.toLowerCase().replace(/\s+/g, '')
  const t = text.toLowerCase()
  if (!q) return 0
  let score = 0
  let ti = 0
  let streak = 0
  for (const ch of q) {
    const found = t.indexOf(ch, ti)
    if (found === -1) return null
    streak = found === ti ? streak + 1 : 0
    score += 1 + streak * 2 + (found === 0 || t[found - 1] === ' ' ? 3 : 0)
    ti = found + 1
  }
  return score - t.length * 0.01
}
