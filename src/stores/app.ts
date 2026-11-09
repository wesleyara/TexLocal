import { defineStore } from 'pinia'

export const useAppStore = defineStore('app', {
  state: () => ({ name: 'TexLocal', version: __APP_VERSION__ }),
})
