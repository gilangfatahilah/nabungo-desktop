import { toast } from 'vue-sonner'
import { i18n } from '@/i18n'
import { IpcFailure } from '@/ipc'
import { errorMessage, type Translate } from '@/lib/errors'

/**
 * Jalankan aksi IPC; error diterjemahkan dan ditampilkan sebagai toast. `recovery_required` tidak
 * di-toast: handler pemulihan sudah pindah ke layar yang menjelaskannya.
 */
export async function withErrorToast(fn: () => Promise<unknown>): Promise<void> {
  try {
    await fn()
  } catch (e) {
    if (e instanceof IpcFailure && e.error.code === 'recovery_required') return
    toast.error(errorMessage(e, i18n.global.t as unknown as Translate))
  }
}
