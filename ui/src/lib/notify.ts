import { toast } from 'vue-sonner'
import { i18n } from '@/i18n'
import { errorMessage, type Translate } from '@/lib/errors'

/** Jalankan aksi IPC; error diterjemahkan dan ditampilkan sebagai toast. */
export async function withErrorToast(fn: () => Promise<unknown>): Promise<void> {
  try {
    await fn()
  } catch (e) {
    toast.error(errorMessage(e, i18n.global.t as unknown as Translate))
  }
}
