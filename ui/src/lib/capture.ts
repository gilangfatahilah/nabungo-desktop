// Draft email capture (EML-03) → state form transaksi. Validasi tetap di Rust; helper ini hanya
// mengosongkan referensi yang tidak bisa dipakai agar form meminta user memilih ulang (K6).
import type { AccountDto, CaptureDto, CategoryDto } from '@/ipc'
import { emptyForm, type TxnFormState } from './txnForm'

type Ctx = { accounts: AccountDto[]; categories: CategoryDto[] }

const activeAccount = (ctx: Ctx, accountId: string | null) =>
  accountId !== null && ctx.accounts.some((a) => a.id === accountId && !a.archived) ? accountId : null

/** Kategori aktif yang sejenis dengan draft; transfer tidak berkategori. */
function usableCategory(c: CaptureDto, ctx: Ctx): string | null {
  if (c.kind === 'transfer' || c.categoryId === null) return null
  const hit = ctx.categories.find((x) => x.id === c.categoryId)
  return hit && !hit.archived && hit.kind === c.kind ? hit.id : null
}

/** Draft → state form; akun/kategori arsip atau salah jenis dikosongkan (K6). */
export function fromCapture(c: CaptureDto, ctx: Ctx & { feeCategoryId: string | null }): TxnFormState {
  const s = emptyForm(c.kind, { accountId: activeAccount(ctx, c.accountId), feeCategoryId: ctx.feeCategoryId, today: c.date })
  return {
    ...s,
    amount: c.amount,
    toAccountId: c.kind === 'transfer' ? activeAccount(ctx, c.toAccountId) : null,
    categoryId: usableCategory(c, ctx),
    payee: c.payee ?? '',
  }
}

/** Siap dicatat sekali klik (K5): referensi lengkap & aktif, tanpa kandidat duplikat. */
export function isReady(c: CaptureDto, ctx: Ctx): boolean {
  const from = activeAccount(ctx, c.accountId)
  if (from === null || c.duplicates.length > 0) return false
  if (c.kind === 'transfer') {
    const to = activeAccount(ctx, c.toAccountId)
    return to !== null && to !== from
  }
  return usableCategory(c, ctx) !== null
}
