import id from '@/i18n/id'
import { IpcFailure } from '@/ipc'

/** Subset struktural dari `useI18n().t` / `i18n.global.t` (generik ketatnya tak saling cocok). */
export type Translate = (key: string, named?: Record<string, unknown>) => string

const KNOWN = new Set(Object.keys(id.errors))

/** Terjemahkan error IPC; apa pun selain kode yang dikenal menjadi `errors.internal`. */
export function errorMessage(e: unknown, t: Translate): string {
  if (e instanceof IpcFailure) {
    const code = e.error?.code
    if (typeof code === 'string' && KNOWN.has(code)) {
      return t(`errors.${code}`, { entity: e.error.entity ?? '' })
    }
  }
  return t('errors.internal')
}
