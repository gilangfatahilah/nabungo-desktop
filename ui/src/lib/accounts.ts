import type { AccountDto, AccountGroupDto, AccountKindDto, AccountsOverviewDto } from '@/ipc'

export interface GroupSection {
  /** `null` = "Tanpa grup" (atau daftar arsip). */
  group: AccountGroupDto | null
  accounts: AccountDto[]
  subtotal: number
}

export const ACCOUNT_KINDS: AccountKindDto[] = ['cash', 'bank', 'ewallet', 'investment', 'receivable', 'credit_card', 'loan']

/** Ikon default per jenis akun bila akun tidak punya ikon sendiri. */
export const KIND_ICONS: Record<AccountKindDto, string> = {
  cash: 'banknote',
  bank: 'landmark',
  ewallet: 'smartphone',
  investment: 'trending-up',
  receivable: 'hand-coins',
  credit_card: 'credit-card',
  loan: 'building-2',
}

/** Sama dengan `AccountKind::is_liability` di Rust: saldonya negatif saat berutang. */
export function isLiability(kind: AccountKindDto): boolean {
  return kind === 'credit_card' || kind === 'loan'
}

/** Form meminta utang sebagai angka positif; saldo awal liabilitas disimpan negatif. */
export const openingSign = (kind: AccountKindDto) => (isLiability(kind) ? -1 : 1)

const bySort = <T extends { sortOrder: number }>(a: T, b: T) => a.sortOrder - b.sortOrder

function section(group: AccountGroupDto | null, accounts: AccountDto[]): GroupSection {
  const sorted = [...accounts].sort(bySort)
  return { group, accounts: sorted, subtotal: sorted.reduce((s, a) => s + a.balance, 0) }
}

/** Kelompokkan akun per group (urut `sortOrder`); "Tanpa grup" terakhir dan hanya bila berisi. */
export function groupAccounts(o: AccountsOverviewDto, opts: { archived: boolean }): GroupSection[] {
  if (opts.archived) {
    const archived = o.accounts.filter((a) => a.archived)
    return archived.length ? [section(null, archived)] : []
  }
  const active = o.accounts.filter((a) => !a.archived)
  const known = new Set(o.groups.map((g) => g.id))
  const sections = [...o.groups].sort(bySort).map((g) => section(g, active.filter((a) => a.groupId === g.id)))
  const ungrouped = active.filter((a) => a.groupId === null || !known.has(a.groupId))
  if (ungrouped.length) sections.push(section(null, ungrouped))
  return sections
}

/** Urutan ID baru untuk group tujuan setelah `accountId` dipindah ke `index` (seret atau tombol atas/bawah). */
export function moveAccount(sections: GroupSection[], accountId: string, targetGroupId: string | null, index: number): string[] {
  const target = sections.find((s) => (s.group?.id ?? null) === targetGroupId)
  const ids = (target?.accounts ?? []).map((a) => a.id).filter((id) => id !== accountId)
  ids.splice(Math.max(0, Math.min(index, ids.length)), 0, accountId)
  return ids
}

/** Warna saldo: negatif (utang kartu kredit, overdraft) tampil sebagai expense. */
export function balanceTone(a: AccountDto): 'neutral' | 'expense' {
  return a.balance < 0 ? 'expense' : 'neutral'
}
