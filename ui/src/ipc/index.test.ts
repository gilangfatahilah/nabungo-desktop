import { describe, expect, it } from 'vitest'
import { appInfo, getUiSettings, IpcFailure, parseAmount, setTheme } from './index'

// jsdom tanpa Tauri: semua fungsi jatuh ke implementasi browser.
describe('ipc facade (browser fallback)', () => {
  it('appInfo returns browser-dev', async () => {
    expect((await appInfo()).version).toBe('browser-dev')
  })

  it('setTheme persists for getUiSettings', async () => {
    await setTheme('dark')
    expect((await getUiSettings()).theme).toBe('dark')
  })

  it('parseAmount rejects malformed input with IpcFailure amount_invalid', async () => {
    const err = await parseAmount('25.00').catch((e: unknown) => e)
    expect(err).toBeInstanceOf(IpcFailure)
    expect((err as IpcFailure).error.code).toBe('amount_invalid')
  })

  it('parseAmount accepts integer input', async () => {
    expect(await parseAmount('15000')).toBe(15000)
  })
})
