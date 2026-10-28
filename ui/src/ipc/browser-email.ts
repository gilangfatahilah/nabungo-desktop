// Email capture mode browser (K14): port aturan `crates/app/src/email_settings.rs` dan
// `email_runner.rs` tanpa jaringan/keychain — ubah bersamaan. Password `wrong` → login gagal,
// label `Missing` → label tidak ada; data capture di browser menunggu 9c.
import type { EmailStatusDto, EmailSyncDone, EmailSyncProgress, ErrorCode, IpcError, SyncSummaryDto } from './bindings'

type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: IpcError }

export type EmailSyncHandlers = { progress?: (p: EmailSyncProgress) => void; done?: (d: EmailSyncDone) => void }

const DEFAULT_MAILBOX = 'Finance'
const ok = <T>(data: T): Result<T> => ({ status: 'ok', data })
const fail = (code: ErrorCode): Result<never> => ({ status: 'error', error: { code, entity: null } })

/** `normalize_address`: trim + lowercase, ≤ 254 byte, tanpa whitespace, tepat satu `@`. */
function normalizeAddress(input: string): string | null {
  const address = input.trim().toLowerCase()
  const at = address.indexOf('@')
  const valid =
    new TextEncoder().encode(address).length <= 254 &&
    !/\s/u.test(address) &&
    at > 0 &&
    at < address.length - 1 &&
    !address.slice(at + 1).includes('@')
  return valid ? address : null
}

/** `normalize_app_password`: spasi pengelompokan Gmail bukan bagian password. */
const normalizePassword = (input: string) => input.replace(/\s/gu, '')

export function createBrowserEmail() {
  const settings = { enabled: false, address: null as string | null, mailbox: DEFAULT_MAILBOX, lastSyncAt: null as string | null }
  const keychain = new Map<string, string>()
  let lastError: ErrorCode | null = null
  const listeners = new Set<EmailSyncHandlers>()

  return {
    emailStatus: async (): Promise<Result<EmailStatusDto>> =>
      ok({ ...settings, running: false, lastError, paused: false }),

    connectEmail: async (address: string, mailbox: string, password: string | null): Promise<Result<null>> => {
      const addr = normalizeAddress(address)
      if (addr === null) return fail('email_address_invalid')
      const label = mailbox.trim()
      if (label === '') return fail('email_mailbox_empty')
      const entered = password === null ? '' : normalizePassword(password)
      const pw = entered !== '' ? entered : keychain.get(addr)
      if (pw === undefined) return fail('email_not_configured')
      if (pw === 'wrong') return fail('email_auth_failed')
      if (label === 'Missing') return fail('email_mailbox_not_found')
      keychain.set(addr, pw)
      if (settings.address !== null && settings.address !== addr) keychain.delete(settings.address)
      Object.assign(settings, { address: addr, mailbox: label, enabled: true })
      lastError = null
      return ok(null)
    },

    disconnectEmail: async (): Promise<Result<null>> => {
      settings.enabled = false
      if (settings.address !== null) keychain.delete(settings.address)
      lastError = null
      return ok(null)
    },

    syncEmail: async (): Promise<Result<SyncSummaryDto>> => {
      if (!settings.enabled || settings.address === null || !keychain.has(settings.address)) return fail('email_not_configured')
      const summary = { newItems: 0, unparsed: 0, seen: 0 }
      settings.lastSyncAt = new Date().toISOString()
      lastError = null
      for (const h of listeners) h.done?.({ summary, error: null })
      return ok(summary)
    },

    subscribe: (h: EmailSyncHandlers): (() => void) => {
      listeners.add(h)
      return () => listeners.delete(h)
    },
  }
}

export const browserEmail = createBrowserEmail()
