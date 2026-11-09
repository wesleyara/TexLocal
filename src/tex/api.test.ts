import { describe, expect, it } from 'vitest'
import { detectTex, distributionName, type Distribution } from './api'

const base: Distribution = {
  kind: 'texLive',
  year: 2025,
  binDir: '/usr/local/texlive/2025/bin/x86_64-linux',
  engines: ['pdflatex'],
  hasTlmgr: true,
  hasLatexmk: true,
}

describe('tex api', () => {
  it('nomeia distribuições com e sem ano', () => {
    expect(distributionName(base)).toBe('TeX Live 2025')
    expect(distributionName({ ...base, kind: 'mikTex', year: null })).toBe('MiKTeX')
    expect(distributionName({ ...base, kind: 'macTex', year: 2024 })).toBe('MacTeX 2024')
  })

  it('no navegador (sem Tauri) a detecção devolve lista vazia', async () => {
    expect(await detectTex()).toEqual([])
  })
})
