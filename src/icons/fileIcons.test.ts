import { describe, expect, it } from 'vitest'
import { allReferencedKeys, fileIconKey, folderIconKey } from './fileIcons'
import { ICONS } from './generated'

describe('ícones de arquivo e pasta', () => {
  it('escolhe pela extensão, sem diferenciar maiúsculas', () => {
    expect(fileIconKey('main.tex')).toBe('mit:tex')
    expect(fileIconKey('Refs.BIB')).toBe('mit:bibliography')
    expect(fileIconKey('figura.PNG')).toBe('mit:image')
    expect(fileIconKey('main.pdf')).toBe('mit:pdf')
  })

  it('prioriza o nome conhecido sobre a extensão', () => {
    expect(fileIconKey('texlocal.toml')).toBe('mit:tune')
    expect(fileIconKey('README.md')).toBe('mit:readme')
    expect(fileIconKey('.gitignore')).toBe('mit:git')
  })

  it('usa o ícone genérico quando não conhece o tipo', () => {
    expect(fileIconKey('Makefile')).toBe('mit:document')
    expect(fileIconKey('arquivo.xyz')).toBe('mit:document')
  })

  it('escolhe pastas por nome e estado', () => {
    expect(folderIconKey('src', false)).toBe('mit:folder-src')
    expect(folderIconKey('Figures', true)).toBe('mit:folder-images-open')
    expect(folderIconKey('qualquer', false)).toBe('mit:folder-base')
  })

  it('todo ícone referenciado existe no conjunto gerado', () => {
    const missing = allReferencedKeys().filter((k) => !(k in ICONS))
    expect(missing).toEqual([])
  })
})
