import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { LIMITS, useUiStore } from './ui'

describe('ui store', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('alterna sidebar e painel inferior', () => {
    const ui = useUiStore()
    expect(ui.layout.sidebarVisible).toBe(true)
    ui.toggleSidebar()
    ui.toggleBottom()
    expect(ui.layout.sidebarVisible).toBe(false)
    expect(ui.layout.bottomVisible).toBe(true)
  })

  it('limita os tamanhos dos painéis', () => {
    const ui = useUiStore()
    ui.resize('sidebarWidth', 5)
    expect(ui.layout.sidebarWidth).toBe(LIMITS.sidebarWidth.min)
    ui.resize('pdfWidth', 99999)
    expect(ui.layout.pdfWidth).toBe(LIMITS.pdfWidth.max)
  })

  it('persiste o layout por projeto', async () => {
    const ui = useUiStore()
    ui.setProject('a')
    ui.setViewMode('pdf')
    await new Promise((r) => setTimeout(r, 0))
    ui.setProject('b')
    expect(ui.layout.viewMode).toBe('split')
    ui.setProject('a')
    expect(ui.layout.viewMode).toBe('pdf')
  })
})
