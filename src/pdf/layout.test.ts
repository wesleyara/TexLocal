import { describe, expect, it } from 'vitest'
import {
  anchorAt,
  CSS_UNITS,
  currentPage,
  layoutPages,
  percentOf,
  scaleFor,
  scrollForAnchor,
  stepZoom,
  visibleRange,
} from './layout'

const A4 = { w: 595, h: 842 }

describe('pdf layout', () => {
  it('ajusta à largura e à página', () => {
    expect(scaleFor({ kind: 'fitWidth' }, A4, { w: 595, h: 300 })).toBeCloseTo(1)
    // Caixa baixa: a altura limita.
    expect(scaleFor({ kind: 'fitPage' }, A4, { w: 1190, h: 842 })).toBeCloseTo(1)
    expect(scaleFor({ kind: 'fitPage' }, A4, { w: 595, h: 421 })).toBeCloseTo(0.5)
  })

  it('percentual usa 96/72 e respeita limites', () => {
    expect(scaleFor({ kind: 'percent', value: 100 }, A4, { w: 1, h: 1 })).toBeCloseTo(CSS_UNITS)
    expect(percentOf(scaleFor({ kind: 'percent', value: 150 }, A4, { w: 1, h: 1 }))).toBe(150)
    expect(percentOf(scaleFor({ kind: 'percent', value: 9999 }, A4, { w: 1, h: 1 }))).toBe(500)
    expect(percentOf(scaleFor({ kind: 'percent', value: 1 }, A4, { w: 1, h: 1 }))).toBe(25)
  })

  it('passos de zoom a partir de qualquer valor', () => {
    expect(stepZoom(100, 1)).toBe(125)
    expect(stepZoom(100, -1)).toBe(75)
    expect(stepZoom(110, 1)).toBe(125)
    expect(stepZoom(110, -1)).toBe(100)
    expect(stepZoom(500, 1)).toBe(500)
    expect(stepZoom(25, -1)).toBe(25)
  })

  const heights = [100, 100, 100, 100]
  const layout = layoutPages(heights, 10, 5)

  it('empilha as páginas com espaçamento', () => {
    expect(layout.tops).toEqual([5, 115, 225, 335])
    expect(layout.total).toBe(5 + 4 * 100 + 3 * 10 + 5)
    expect(layoutPages([], 10, 5).total).toBe(0)
  })

  it('calcula o intervalo visível (com margem)', () => {
    expect(visibleRange(layout, heights, 0, 100)).toEqual([0, 0])
    expect(visibleRange(layout, heights, 100, 100)).toEqual([0, 1])
    expect(visibleRange(layout, heights, 0, 100, 150)).toEqual([0, 2])
    expect(visibleRange(layout, [], 0, 100)).toBeNull()
    expect(visibleRange(layout, heights, 5000, 100)).toBeNull()
  })

  it('página atual e âncora de zoom', () => {
    expect(currentPage(layout, heights, 0, 90)).toBe(0)
    expect(currentPage(layout, heights, 150, 90)).toBe(1)
    expect(currentPage(layout, heights, 5000, 90)).toBe(3)

    const anchor = anchorAt(layout, heights, 165)
    expect(anchor.page).toBe(1)
    expect(anchor.fraction).toBeCloseTo(0.5)
    // Ao dobrar o zoom, a mesma posição relativa continua no topo.
    const big = layoutPages([200, 200, 200, 200], 10, 5)
    expect(scrollForAnchor(big, [200, 200, 200, 200], anchor)).toBeCloseTo(215 + 100)
  })

  it('300 páginas: o intervalo visível é pequeno', () => {
    const hs = Array.from({ length: 300 }, () => 1123)
    const l = layoutPages(hs, 8, 8)
    const [first, last] = visibleRange(l, hs, 150 * 1131, 800, 1200)!
    expect(last - first).toBeLessThanOrEqual(3)
    expect(first).toBeGreaterThanOrEqual(148)
  })
})
