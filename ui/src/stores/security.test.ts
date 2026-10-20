import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { IpcFailure } from '@/ipc'
import { useSecurityStore } from './security'

const PW = 'rahasia-123'

const code = (e: unknown) => (e instanceof IpcFailure ? e.error.code : String(e))

describe('security store (fallback browser)', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('enable lalu lock lalu unlock', async () => {
    const s = useSecurityStore()
    await s.refresh()
    expect(s.encrypted).toBe(false)
    const rk = await s.enable(PW)
    expect(rk).toMatch(/^([0-9A-Z]{4}-){12}[0-9A-Z]{4}$/)
    expect(s.encrypted).toBe(true)
    expect(s.hasKeyfile).toBe(true)
    await s.lock()
    expect(s.locked).toBe(true)
    await s.unlock(PW)
    expect(s.locked).toBe(false)
    expect(s.autoLockMinutes).toBe(5)
    await s.disable(PW)
    expect(s.encrypted).toBe(false)
  })

  it('unlock salah → wrong_password, tetap locked', async () => {
    const s = useSecurityStore()
    await s.enable(PW)
    await s.lock()
    await expect(s.unlock('salah-1234').catch(code)).resolves.toBe('wrong_password')
    expect(s.locked).toBe(true)
    await s.unlock(PW)
    await s.disable(PW)
  })

  it('recovery mengganti password', async () => {
    const s = useSecurityStore()
    const rk = await s.enable(PW)
    await s.lock()
    await expect(s.unlockWithRecovery('XXXX', 'baru-12345').catch(code)).resolves.toBe('invalid_recovery_key')
    // Huruf kecil, spasi, dan O/L untuk 0/1 tetap diterima, seperti `parse_recovery_key`.
    const typed = rk.toLowerCase().replace(/-/g, ' ').replace(/0/g, 'o').replace(/1/g, 'l')
    await s.unlockWithRecovery(typed, 'baru-12345')
    expect(s.locked).toBe(false)
    await s.lock()
    await expect(s.unlock(PW).catch(code)).resolves.toBe('wrong_password')
    await s.unlock('baru-12345')
    await s.changePassword('baru-12345', 'lagi-12345')
    await expect(s.disable('baru-12345').catch(code)).resolves.toBe('wrong_password')
    await s.disable('lagi-12345')
  })

  it('auto-lock: hanya durasi yang tersedia', async () => {
    const s = useSecurityStore()
    await s.setAutoLock(15)
    expect(s.autoLockMinutes).toBe(15)
    await expect(s.setAutoLock(7).catch(code)).resolves.toBe('invalid_auto_lock')
    expect(s.autoLockMinutes).toBe(15)
    await s.setAutoLock(5)
  })

  it('aturan password & status mengikuti Rust', async () => {
    const s = useSecurityStore()
    await expect(s.enable('1234567').catch(code)).resolves.toBe('password_too_short')
    await expect(s.lock().catch(code)).resolves.toBe('encryption_state')
    await expect(s.disable(PW).catch(code)).resolves.toBe('encryption_state')
    await s.enable(PW)
    await expect(s.enable(PW).catch(code)).resolves.toBe('encryption_state')
    await s.disable(PW)
  })
})
