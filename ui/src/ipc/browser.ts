// Implementasi pengganti saat UI dijalankan di browser biasa (tanpa backend Tauri).
import type { AppInfo, IpcError, UiSettingsDto } from './bindings'
import { parseAmountBrowser } from './browser-amount'
import { browserLedger, browserSecurity, isBrowserLocked } from './browser-ledger'

type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: IpcError }

const settings: UiSettingsDto = { theme: 'system', locale: 'id', autoLockMinutes: 5 }

/** Sama dengan `AUTO_LOCK_CHOICES` di `crates/app/src/settings.rs`. */
const AUTO_LOCK_CHOICES = [1, 5, 10, 15, 30]

const ok = <T>(data: T): Result<T> => ({ status: 'ok', data })

type Command = (...args: never[]) => Promise<unknown>

// Seperti Rust: command yang tidak butuh ledger tetap jalan saat terkunci.
const OPEN_WHEN_LOCKED = new Set([
  'appInfo', 'startupReport', 'parseAmount', 'listBackups',
  'securityStatus', 'unlock', 'unlockWithRecovery', 'lock',
])

function lockGuard<T extends Record<string, Command>>(cmds: T): T {
  const locked: Result<never> = { status: 'error', error: { code: 'locked', entity: null } }
  return Object.fromEntries(
    Object.entries(cmds).map(([name, fn]) => [
      name,
      OPEN_WHEN_LOCKED.has(name) ? fn : (...args: never[]) => (isBrowserLocked() ? Promise.resolve(locked) : fn(...args)),
    ]),
  ) as T
}

export const browserCommands = lockGuard({
  ...browserLedger,
  ...browserSecurity,
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
    if (!AUTO_LOCK_CHOICES.includes(minutes)) return { status: 'error', error: { code: 'invalid_auto_lock', entity: null } }
    settings.autoLockMinutes = minutes
    return ok(null)
  },
})
