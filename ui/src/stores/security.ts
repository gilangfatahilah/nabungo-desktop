import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Router } from 'vue-router'
import {
  changePassword as ipcChangePassword,
  disableEncryption,
  enableEncryption,
  getUiSettings,
  lock as ipcLock,
  securityStatus,
  setAutoLockMinutes,
  setLockedHandler,
  unlock as ipcUnlock,
  unlockWithRecovery as ipcUnlockWithRecovery,
} from '@/ipc'
import { reloadApp } from '@/lib/reload'

export const useSecurityStore = defineStore('security', () => {
  const encrypted = ref(false)
  const locked = ref(false)
  const hasKeyfile = ref(false)
  const autoLockMinutes = ref(5)

  /** Status dari backend; durasi auto-lock hanya terbaca saat tidak terkunci. */
  async function refresh() {
    const s = await securityStatus()
    encrypted.value = s.encrypted
    locked.value = s.locked
    hasKeyfile.value = s.hasKeyfile
    if (!s.locked) autoLockMinutes.value = (await getUiSettings()).autoLockMinutes
  }

  async function unlock(password: string) {
    await ipcUnlock(password)
    await refresh()
  }

  async function unlockWithRecovery(recoveryKey: string, newPassword: string) {
    await ipcUnlockWithRecovery(recoveryKey, newPassword)
    await refresh()
  }

  async function lock() {
    await ipcLock()
    await refresh()
  }

  /** Mengembalikan recovery key untuk ditampilkan sekali. */
  async function enable(password: string): Promise<string> {
    const key = await enableEncryption(password)
    await refresh()
    return key
  }

  async function disable(password: string) {
    await disableEncryption(password)
    await refresh()
  }

  async function changePassword(oldPassword: string, newPassword: string) {
    await ipcChangePassword(oldPassword, newPassword)
    await refresh()
  }

  async function setAutoLock(minutes: number) {
    await setAutoLockMinutes(minutes)
    await refresh()
  }

  /**
   * Pindah ke layar kunci lalu muat ulang UI, agar data store lain tidak tertinggal di memori
   * di balik layar kunci. Sudah di `unlock` → cukup tandai terkunci (tidak ada loop reload).
   */
  async function showLockScreen(router: Router) {
    locked.value = true
    const current = router.currentRoute.value
    if (current.name === 'unlock') return
    const redirect = current.name === 'dashboard' || current.name === undefined ? undefined : current.fullPath
    await router.replace({ name: 'unlock', query: redirect ? { redirect } : {} })
    reloadApp()
  }

  /** Error `locked` dari command apa pun → layar kunci. */
  function installLockHandler(router: Router) {
    setLockedHandler(() => void showLockScreen(router))
  }

  return {
    encrypted, locked, hasKeyfile, autoLockMinutes,
    refresh, unlock, unlockWithRecovery, lock, enable, disable, changePassword, setAutoLock,
    showLockScreen, installLockHandler,
  }
})
