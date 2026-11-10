import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
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

export type ProfileId = 'minimal' | 'recommended' | 'full'

export interface ProfileInfo {
  id: ProfileId
  approxSizeMb: number
}

export interface DiskInfo {
  path: string
  freeBytes: number | null
}

export type InstallPhase = 'preflight' | 'download' | 'verify' | 'extract' | 'install' | 'finalize'

export interface InstallEvent {
  phase: InstallPhase
  progress: number | null
  message: string
}

export interface InstallResult {
  year: number
  tree: string
  binDir: string
}

export interface InstallError {
  kind: string
  /** Chave de tradução da mensagem amigável. */
  i18nKey: string
  details: string
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

/** Espaço necessário (bytes) para um perfil: tamanho instalado + 25% (igual ao backend). */
export function requiredBytes(approxSizeMb: number): number {
  return Math.floor((approxSizeMb * 1024 * 1024 * 5) / 4)
}

/** Tamanho aproximado de perfil em MB decimais: 150 -> "150 MB", 5000 -> "5 GB". */
export function formatProfileSize(mb: number): string {
  return mb >= 1000 ? `${+(mb / 1000).toFixed(1)} GB` : `${mb} MB`
}

export function formatBytes(bytes: number): string {
  const gb = bytes / 1024 ** 3
  if (gb >= 1) return `${gb.toFixed(1)} GB`
  return `${Math.round(bytes / 1024 ** 2)} MB`
}

/** Normaliza qualquer falha em um InstallError exibível. */
export function toInstallError(e: unknown): InstallError {
  if (typeof e === 'object' && e !== null && 'i18nKey' in e) return e as InstallError
  return { kind: 'unknown', i18nKey: 'installer.error.installFailed', details: String(e) }
}

/** Detecção e instalação só existem no app desktop; no navegador (dev) devolvem vazio. */
export async function detectTex(): Promise<Distribution[]> {
  if (!isTauri()) return []
  return invoke<Distribution[]>('detect_tex')
}

export async function texProfiles(): Promise<ProfileInfo[]> {
  if (!isTauri()) {
    return [
      { id: 'minimal', approxSizeMb: 150 },
      { id: 'recommended', approxSizeMb: 800 },
      { id: 'full', approxSizeMb: 5000 },
    ]
  }
  return invoke<ProfileInfo[]>('tex_profiles')
}

export async function texDiskInfo(): Promise<DiskInfo> {
  if (!isTauri()) return { path: '', freeBytes: null }
  return invoke<DiskInfo>('tex_disk_info')
}

export async function installTex(profile: ProfileId, mirror?: string): Promise<InstallResult> {
  if (!isTauri()) {
    throw { kind: 'unavailable', i18nKey: 'installer.error.unavailable', details: '' }
  }
  return invoke<InstallResult>('tex_install', { profile, mirror: mirror ?? null })
}

export async function cancelInstall(): Promise<void> {
  if (isTauri()) await invoke('tex_install_cancel')
}

export async function onInstallEvent(cb: (e: InstallEvent) => void): Promise<UnlistenFn> {
  if (!isTauri()) return () => {}
  return listen<InstallEvent>('tex-install', (ev) => cb(ev.payload))
}
