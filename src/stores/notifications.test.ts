import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useNotificationsStore } from './notifications'

describe('notifications store', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    setActivePinia(createPinia())
  })
  afterEach(() => vi.useRealTimers())

  it('toast expira mas permanece no histórico', () => {
    const s = useNotificationsStore()
    s.notify('info', 'olá', 1000)
    expect(s.toasts).toHaveLength(1)
    vi.advanceTimersByTime(1000)
    expect(s.toasts).toHaveLength(0)
    expect(s.history).toHaveLength(1)
  })

  it('tarefas têm progresso limitado e podem ser canceladas', () => {
    const s = useNotificationsStore()
    const cancel = vi.fn()
    const id = s.startTask('download', cancel)
    s.updateTask(id, 3)
    expect(s.tasks[0].progress).toBe(1)
    s.cancelTask(id)
    expect(cancel).toHaveBeenCalledOnce()
    expect(s.tasks).toHaveLength(0)
  })
})
