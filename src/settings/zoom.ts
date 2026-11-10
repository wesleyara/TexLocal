import { isTauri } from '../tauri'
import { ZOOM_MAX, ZOOM_MIN } from './schema'

/** Cada passo de zoom multiplica a escala por 1,2 (como o `window.zoomLevel` do VS Code). */
export function zoomFactor(level: number): number {
  return Math.round(1.2 ** level * 1000) / 1000
}

export function clampZoom(level: number): number {
  return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, level))
}

export async function applyZoom(level: number): Promise<void> {
  const factor = zoomFactor(level)
  if (isTauri()) {
    try {
      const { getCurrentWebview } = await import('@tauri-apps/api/webview')
      await getCurrentWebview().setZoom(factor)
      return
    } catch {
      // sem permissão/API: cai no zoom por CSS
    }
  }
  document.documentElement.style.setProperty('zoom', String(factor))
}
