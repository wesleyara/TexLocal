import type { CommitInfo } from './api'

/** Intervalo sem commits que separa duas "sessões de edição". */
export const SESSION_GAP_SECONDS = 30 * 60

export interface Session {
  /** Chave estável: id do commit mais recente da sessão. */
  key: string
  day: string
  commits: CommitInfo[]
}

/** Dia local (`YYYY-MM-DD`) de um timestamp em segundos. */
export function dayKey(seconds: number): string {
  const d = new Date(seconds * 1000)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`
}

/**
 * Agrupa commits (mais recentes primeiro) em sessões: commits consecutivos no mesmo dia, com
 * intervalo menor que `SESSION_GAP_SECONDS`. Versões nomeadas ficam sempre numa sessão própria
 * para destacá-las na timeline.
 */
export function groupSessions(commits: CommitInfo[]): Session[] {
  const out: Session[] = []
  for (const c of commits) {
    const day = dayKey(c.time)
    const last = out[out.length - 1]
    const prev = last?.commits[last.commits.length - 1]
    const joins =
      last &&
      prev &&
      last.day === day &&
      prev.time - c.time < SESSION_GAP_SECONDS &&
      c.tags.length === 0 &&
      last.commits[0].tags.length === 0
    if (joins) last.commits.push(c)
    else out.push({ key: c.id, day, commits: [c] })
  }
  return out
}

/** Resumo exibível de um commit: o nome da versão, se houver, ou a mensagem. */
export function commitTitle(c: CommitInfo): string {
  return c.tags[0]?.label ?? c.summary
}
