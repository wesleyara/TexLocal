import { defineStore } from 'pinia'
import { ref } from 'vue'
import { t } from '../i18n'
import {
  cancelInstall,
  installTex,
  onInstallEvent,
  toInstallError,
  type InstallError,
  type InstallEvent,
  type ProfileId,
} from '../tex/api'
import { useNotificationsStore } from './notifications'
import { useSettingsStore } from './settings'

export type InstallStatus = 'idle' | 'running' | 'done' | 'error'

/** Controla a instalação do TeX gerenciado; sobrevive ao fechamento do assistente. */
export const useTexInstallStore = defineStore('texInstall', () => {
  const status = ref<InstallStatus>('idle')
  const event = ref<InstallEvent | null>(null)
  const error = ref<InstallError | null>(null)

  async function start(profile: ProfileId): Promise<boolean> {
    if (status.value === 'running') return false
    const notifications = useNotificationsStore()
    const settings = useSettingsStore()

    status.value = 'running'
    event.value = null
    error.value = null
    const taskId = notifications.startTask(t('installer.task'), () => void cancelInstall())
    const unlisten = await onInstallEvent((e) => {
      event.value = e
      notifications.updateTask(taskId, e.progress)
    })

    try {
      const result = await installTex(profile, settings.effective.texMirror)
      await settings.update({ texSource: 'managed', texBinDir: result.binDir })
      status.value = 'done'
      notifications.notify('success', t('installer.done', { year: result.year }))
      return true
    } catch (e) {
      error.value = toInstallError(e)
      status.value = 'error'
      if (error.value.kind !== 'cancelled') {
        notifications.notify('error', t(error.value.i18nKey))
      }
      return false
    } finally {
      unlisten()
      notifications.finishTask(taskId)
    }
  }

  async function cancel() {
    await cancelInstall()
  }

  function reset() {
    if (status.value !== 'running') {
      status.value = 'idle'
      event.value = null
      error.value = null
    }
  }

  return { status, event, error, start, cancel, reset }
})
