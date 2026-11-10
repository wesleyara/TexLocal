/** Matemática de layout do visualizador de PDF (sem DOM, testável). */

/** 1 ponto PDF = 96/72 px CSS a 100%. */
export const CSS_UNITS = 96 / 72

export type ZoomMode =
  { kind: 'fitWidth' } | { kind: 'fitPage' } | { kind: 'percent'; value: number }

export const ZOOM_STEPS = [25, 50, 75, 100, 125, 150, 200, 300, 400, 500]
export const MIN_ZOOM = ZOOM_STEPS[0]
export const MAX_ZOOM = ZOOM_STEPS[ZOOM_STEPS.length - 1]

export const clampPercent = (p: number): number => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, p))

export interface Size {
  w: number
  h: number
}

/**
 * Escala (px CSS por ponto PDF) de um modo de zoom. `page` é a maior página (pontos) e
 * `box` a área disponível (px CSS, já sem margens).
 */
export function scaleFor(mode: ZoomMode, page: Size, box: Size): number {
  if (page.w <= 0 || page.h <= 0) return CSS_UNITS
  switch (mode.kind) {
    case 'fitWidth':
      return Math.max(box.w, 50) / page.w
    case 'fitPage':
      return Math.max(Math.min(Math.max(box.w, 50) / page.w, Math.max(box.h, 50) / page.h), 0.05)
    case 'percent':
      return (clampPercent(mode.value) / 100) * CSS_UNITS
  }
}

/** Percentual exibido para uma escala. */
export const percentOf = (scale: number): number => Math.round((scale / CSS_UNITS) * 100)

/** Próximo passo de zoom (para + / -) a partir de um percentual arbitrário. */
export function stepZoom(percent: number, direction: 1 | -1): number {
  const p = Math.round(percent)
  if (direction > 0) return ZOOM_STEPS.find((s) => s > p) ?? MAX_ZOOM
  return [...ZOOM_STEPS].reverse().find((s) => s < p) ?? MIN_ZOOM
}

export interface Layout {
  /** Topo de cada página, em px, a partir do início do conteúdo rolável. */
  tops: number[]
  /** Altura total do conteúdo. */
  total: number
}

/** Posiciona as páginas em coluna única. `heights` já está na escala atual. */
export function layoutPages(heights: number[], gap: number, pad: number): Layout {
  const tops: number[] = []
  let y = pad
  for (const h of heights) {
    tops.push(y)
    y += h + gap
  }
  return { tops, total: Math.max(y - gap + pad, 0) }
}

/** Primeira e última página (índices) que intersectam a janela, com margem em px. */
export function visibleRange(
  layout: Layout,
  heights: number[],
  scrollTop: number,
  viewportHeight: number,
  margin = 0,
): [number, number] | null {
  if (!heights.length) return null
  const from = scrollTop - margin
  const to = scrollTop + viewportHeight + margin
  let first = -1
  let last = -1
  for (let i = 0; i < heights.length; i++) {
    const top = layout.tops[i]
    if (top + heights[i] >= from && top <= to) {
      if (first < 0) first = i
      last = i
    } else if (first >= 0) {
      break
    }
  }
  return first < 0 ? null : [first, last]
}

/** Página (índice) mais visível: a que contém o ponto a 1/3 da janela. */
export function currentPage(layout: Layout, heights: number[], scrollTop: number, vh: number) {
  if (!heights.length) return 0
  const probe = scrollTop + vh / 3
  let found = 0
  for (let i = 0; i < heights.length; i++) {
    if (layout.tops[i] <= probe) found = i
    else break
  }
  return found
}

/** Âncora (página + fração) para manter a posição ao mudar o zoom. */
export function anchorAt(layout: Layout, heights: number[], scrollTop: number) {
  const page = currentPage(layout, heights, scrollTop, 0)
  const h = heights[page] || 1
  return { page, fraction: (scrollTop - layout.tops[page]) / h }
}

export function scrollForAnchor(
  layout: Layout,
  heights: number[],
  anchor: { page: number; fraction: number },
): number {
  const page = Math.min(anchor.page, heights.length - 1)
  if (page < 0) return 0
  return Math.max(layout.tops[page] + anchor.fraction * heights[page], 0)
}
