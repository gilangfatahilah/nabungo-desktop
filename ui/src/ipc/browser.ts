// Implementasi pengganti saat UI dijalankan di browser biasa (tanpa backend Tauri).
import type { AppInfo, IpcError, UiSettingsDto } from './bindings'
import { parseAmountBrowser } from './browser-amount'
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
    const r = parseAmountBrowser(input)
    return 'ok' in r ? ok(r.ok) : { status: 'error', error: { code: r.err, entity: null } }
  },
}
