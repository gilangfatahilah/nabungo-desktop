// Implementasi pengganti saat UI dijalankan di browser biasa (tanpa backend Tauri).
import type { AppInfo, InboxSummaryDto, IpcError, UiSettingsDto } from './bindings'
import { parseAmountBrowser } from './browser-amount'
import { browserEmail } from './browser-email'
import { __enterRecovery, __seedEmailInbox, browserLedger, browserRecovery, browserSecurity, isBrowserLocked, isBrowserRecovering } from './browser-ledger'
import { isFontScale } from '@/lib/fontScale'
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
  emailStatus: browserEmail.emailStatus,
  connectEmail: browserEmail.connectEmail,
  disconnectEmail: browserEmail.disconnectEmail,
  syncEmail: browserEmail.syncEmail,
  inboxSummary: async (): Promise<Result<InboxSummaryDto>> => {
    const counts = await browserLedger.inboxSummary()
    if (counts.status === 'error') return counts
    const email = await browserEmail.emailStatus()
    return ok({ ...counts.data, emailEnabled: email.status === 'ok' && email.data.enabled })
  },
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
  setFontScale: async (scale: number): Promise<Result<null>> => {
    if (!isFontScale(scale)) return { status: 'error', error: { code: 'invalid_font_scale', entity: null } }
    settings.fontScale = scale
    return ok(null)
  },
})

// Hook e2e (Playwright): masuk mode pemulihan / isi Inbox email tanpa backend Tauri.
if (typeof window !== 'undefined') {
  Object.assign(window, { __enterRecovery, __seedEmailInbox })
}
