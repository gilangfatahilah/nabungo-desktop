// Update aplikasi mode browser: port aturan `src-tauri/src/update.rs` tanpa jaringan. Rilis
// terbaru diatur lewat `window.__seedUpdate({ latest, notes, kind })`; `latest` null → sudah
// terbaru, `kind` null → dev/portable (hanya halaman rilis).
import type { ErrorCode, InstallKindDto, InstallOutcomeDto, IpcError, UpdateDownloadProgress, UpdateInfoDto, UpdateStatusDto } from './bindings'

type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: IpcError }

export type UpdateHandlers = { progress?: (p: UpdateDownloadProgress) => void; available?: (latest: string) => void }
export type UpdateSeed = { latest?: string | null; notes?: string; kind?: InstallKindDto | null; fail?: ErrorCode | null }

const CURRENT = '0.4.0'
const SIZE = 12_000_000
const ok = <T>(data: T): Result<T> => ({ status: 'ok', data })
const fail = (code: ErrorCode): Result<never> => ({ status: 'error', error: { code, entity: null } })

const INSTALLER: Record<InstallKindDto, (v: string) => string> = {
  deb: (v) => `Nabungo_${v}_amd64.deb`,
  rpm: (v) => `Nabungo-${v}-1.x86_64.rpm`,
  appimage: (v) => `Nabungo_${v}_amd64.AppImage`,
  nsis: (v) => `Nabungo_${v}_x64-setup.exe`,
  msi: (v) => `Nabungo_${v}_x64_en-US.msi`,
}

export function createBrowserUpdate() {
  let seed: Required<UpdateSeed> = { latest: null, notes: '', kind: null, fail: null }
  let autoCheck = false
  let lastCheck: UpdateInfoDto | null = null
  let downloaded = false
  let downloading = false
  let cancelled = false
  const listeners = new Set<UpdateHandlers>()

  return {
    seed(s: UpdateSeed) {
      seed = { ...seed, ...s }
      lastCheck = null
      downloaded = false
    },

    updateStatus: async (): Promise<Result<UpdateStatusDto>> =>
      ok({ current: CURRENT, autoCheck, installKind: seed.kind, lastCheck, downloaded, downloading }),

    setUpdateAutoCheck: async (enabled: boolean): Promise<Result<null>> => {
      autoCheck = enabled
      return ok(null)
    },

    checkUpdate: async (): Promise<Result<UpdateInfoDto>> => {
      if (seed.fail) return fail(seed.fail)
      const latest = seed.latest ?? CURRENT
      const available = seed.latest !== null && seed.latest !== CURRENT
      lastCheck = {
        current: CURRENT,
        latest,
        available,
        notes: available ? seed.notes : '',
        publishedAt: available ? new Date().toISOString() : null,
        // Seperti Rust: installer hanya bila ada versi baru dan format instalasi dikenal.
        installer: available && seed.kind ? { name: INSTALLER[seed.kind](latest), size: SIZE } : null,
      }
      downloaded = false
      return ok(lastCheck)
    },

    downloadUpdate: async (): Promise<Result<null>> => {
      if (downloading) return fail('update_busy')
      if (!lastCheck?.installer) return fail('update_no_asset')
      downloading = true
      cancelled = false
      try {
        for (let i = 0; i <= 10; i++) {
          if (cancelled) return fail('update_cancelled')
          for (const h of listeners) h.progress?.({ downloaded: (SIZE / 10) * i, total: SIZE })
          await new Promise((r) => setTimeout(r, 0))
        }
        downloaded = true
        return ok(null)
      } finally {
        downloading = false
      }
    },

    cancelUpdateDownload: async (): Promise<void> => {
      cancelled = true
    },

    installUpdate: async (): Promise<Result<InstallOutcomeDto>> => {
      if (!downloaded || !seed.kind) return fail('update_no_asset')
      // AppImage di Rust memulai ulang aplikasi; di browser cukup dianggap terbuka.
      return ok(seed.kind === 'nsis' || seed.kind === 'msi' ? 'exiting' : 'installer_opened')
    },

    openReleasePage: async (): Promise<Result<null>> => ok(null),

    subscribe: (h: UpdateHandlers): (() => void) => {
      listeners.add(h)
      return () => listeners.delete(h)
    },
  }
}

export const browserUpdate = createBrowserUpdate()
