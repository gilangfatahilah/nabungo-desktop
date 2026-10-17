// Implementasi pengganti saat UI dijalankan di browser biasa (tanpa backend Tauri).
import type { AppInfo, IpcError, UiSettingsDto } from './bindings'
import { browserLedger } from './browser-ledger'

type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: IpcError }

const settings: UiSettingsDto = { theme: 'system', locale: 'id' }

const ok = <T>(data: T): Result<T> => ({ status: 'ok', data })

export const browserCommands = {
  ...browserLedger,
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
    const trimmed = input.trim()
    const n = Number(trimmed)
    if (trimmed !== '' && Number.isSafeInteger(n)) return ok(n)
    return { status: 'error', error: { code: 'amount_invalid', entity: null } }
  },
}
