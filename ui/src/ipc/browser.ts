// Implementasi pengganti saat UI dijalankan di browser biasa (tanpa backend Tauri).
import type { AppInfo, IpcError, UiSettingsDto } from './bindings'
import { parseAmountBrowser } from './browser-amount'
import { __enterRecovery, browserLedger, browserRecovery, browserSecurity, isBrowserLocked, isBrowserRecovering } from './browser-ledger'
import { AUTO_LOCK_CHOICES } from '@/lib/security'

type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: IpcError }

const settings: UiSettingsDto = { theme: 'system', locale: 'id', autoLockMinutes: 5, fontScale: 100 }


const ok = <T>(data: T): Result<T> => ({ status: 'ok', data })

type Command = (...args: never[]) => Promise<unknown>

// Seperti Rust: command yang tidak butuh ledger tetap jalan saat pemulihan / terkunci.
const RECOVERY_COMMANDS = ['recoveryStatus', 'pickBackupFile', 'inspectBackupSource', 'recoverFromBackup', 'recoveryStartFresh']
const OPEN_IN_RECOVERY = new Set(['appInfo', 'startupReport', 'parseAmount', 'listBackups', ...RECOVERY_COMMANDS])
const OPEN_WHEN_LOCKED = new Set([
  'appInfo', 'startupReport', 'parseAmount', 'listBackups',
  'securityStatus', 'unlock', 'unlockWithRecovery', 'lock', ...RECOVERY_COMMANDS,
])

const failWith = (code: IpcError['code']): Promise<Result<never>> => Promise.resolve({ status: 'error', error: { code, entity: null } })

function lockGuard<T extends Record<string, Command>>(cmds: T): T {
  return Object.fromEntries(
    Object.entries(cmds).map(([name, fn]) => [
      name,
      (...args: never[]) => {
        // Urutan sama dengan `ledger()` di Rust: pemulihan diperiksa sebelum kunci.
        if (isBrowserRecovering() && !OPEN_IN_RECOVERY.has(name)) return failWith('recovery_required')
        if (isBrowserLocked() && !OPEN_WHEN_LOCKED.has(name)) return failWith('locked')
        return fn(...args)
      },
    ]),
  ) as T
}

export const browserCommands = lockGuard({
  ...browserLedger,
  ...browserSecurity,
  ...browserRecovery,
  appInfo: async (): Promise<AppInfo> => ({ version: 'browser-dev', dataDir: '', portable: false }),
  getUiSettings: async () => ok({ ...settings }),
  setTheme: async (theme: UiSettingsDto['theme']) => {
    settings.theme = theme
    return ok(null)
  },
  setLocale: async (locale: UiSettingsDto['locale']) => {
    settings.locale = locale
    return ok(null)
  },
  // Hanya integer polos; ekspresi (rb/jt, desimal) butuh parser Rust.
  parseAmount: async (input: string): Promise<Result<number>> => {
    const r = parseAmountBrowser(input)
    return 'ok' in r ? ok(r.ok) : { status: 'error', error: { code: r.err, entity: null } }
  },
  setAutoLockMinutes: async (minutes: number): Promise<Result<null>> => {
    if (!(AUTO_LOCK_CHOICES as readonly number[]).includes(minutes)) return { status: 'error', error: { code: 'invalid_auto_lock', entity: null } }
    settings.autoLockMinutes = minutes
    return ok(null)
  },
})

// Hook e2e (Playwright): masuk mode pemulihan tanpa backend Tauri.
if (typeof window !== 'undefined') {
  ;(window as unknown as { __enterRecovery: typeof __enterRecovery }).__enterRecovery = __enterRecovery
}
