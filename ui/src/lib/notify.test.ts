import { beforeEach, describe, expect, it, vi } from 'vitest'

const toastError = vi.fn()
vi.mock('vue-sonner', () => ({ toast: { error: (m: string) => toastError(m) } }))

const { IpcFailure } = await import('@/ipc')
const { withErrorToast } = await import('./notify')

describe('withErrorToast', () => {
  beforeEach(() => toastError.mockClear())

  it('menampilkan error sebagai toast', async () => {
    await withErrorToast(async () => {
      throw new IpcFailure({ code: 'backup_invalid', entity: null })
    })
    expect(toastError).toHaveBeenCalledOnce()
  })

  it('recovery_required tidak di-toast: handler sudah pindah ke layar pemulihan', async () => {
    await withErrorToast(async () => {
      throw new IpcFailure({ code: 'recovery_required', entity: null })
    })
    expect(toastError).not.toHaveBeenCalled()
  })
})
