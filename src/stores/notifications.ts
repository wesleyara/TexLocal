import { defineStore } from 'pinia'
import { ref } from 'vue'

export type NotificationKind = 'info' | 'success' | 'error'

export interface AppNotification {
  id: number
  kind: NotificationKind
  message: string
}

export interface BackgroundTask {
  id: number
  title: string
  /** 0..1, ou null para progresso indeterminado. */
  progress: number | null
  cancel?: () => void
  /** Reabre a tela da tarefa (ex.: o assistente de instalação minimizado). */
  open?: () => void
}

export const useNotificationsStore = defineStore('notifications', () => {
  let seq = 0
  const toasts = ref<AppNotification[]>([])
  const history = ref<AppNotification[]>([])
  const tasks = ref<BackgroundTask[]>([])

  function dismissToast(id: number) {
    toasts.value = toasts.value.filter((t) => t.id !== id)
  }

  function notify(kind: NotificationKind, message: string, timeoutMs = 5000): number {
    const n = { id: ++seq, kind, message }
    toasts.value.push(n)
    history.value.unshift(n)
    if (timeoutMs > 0) setTimeout(() => dismissToast(n.id), timeoutMs)
    return n.id
  }

  function clearHistory() {
    history.value = []
  }

  function startTask(title: string, cancel?: () => void, open?: () => void): number {
    const task: BackgroundTask = { id: ++seq, title, progress: null, cancel, open }
    tasks.value.push(task)
    return task.id
  }

  function updateTask(id: number, progress: number | null) {
    const t = tasks.value.find((x) => x.id === id)
    if (t) t.progress = progress === null ? null : Math.min(1, Math.max(0, progress))
  }

  function finishTask(id: number) {
    tasks.value = tasks.value.filter((t) => t.id !== id)
  }

  function cancelTask(id: number) {
    const t = tasks.value.find((x) => x.id === id)
    t?.cancel?.()
    finishTask(id)
  }

  return {
    toasts,
    history,
    tasks,
    notify,
    dismissToast,
    clearHistory,
    startTask,
    updateTask,
    finishTask,
    cancelTask,
  }
})
