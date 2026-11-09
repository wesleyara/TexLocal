import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useAppStore } from './app'

describe('app store', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('expõe o nome do app', () => {
    expect(useAppStore().name).toBe('TexLocal')
  })
})
