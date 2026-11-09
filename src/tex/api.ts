import { invoke } from '@tauri-apps/api/core'
import { isTauri } from '../tauri'

export type DistKind = 'texLive' | 'mikTex' | 'macTex'

export interface Distribution {
  kind: DistKind
  year: number | null
  binDir: string
  engines: string[]
  hasTlmgr: boolean
  hasLatexmk: boolean
}

export interface ProfileInfo {
  id: 'minimal' | 'recommended' | 'full'
  approxSizeMb: number
}

const KIND_NAMES: Record<DistKind, string> = {
  texLive: 'TeX Live',
  mikTex: 'MiKTeX',
  macTex: 'MacTeX',
}

/** Rótulo curto: "TeX Live 2025" ou "MiKTeX". */
export function distributionName(d: Distribution): string {
  return d.year ? `${KIND_NAMES[d.kind]} ${d.year}` : KIND_NAMES[d.kind]
}

/** Detecção só existe no app desktop; no navegador (dev) devolve vazio. */
export async function detectTex(): Promise<Distribution[]> {
  if (!isTauri()) return []
  return invoke<Distribution[]>('detect_tex')
}
