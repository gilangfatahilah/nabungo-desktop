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

/** Sama dengan `AccountKind::counts_as_liability`: piutang/hutang bersaldo negatif = user berutang. */
export function countsAsLiability(kind: AccountKindDto, balance: number): boolean {
  return isLiability(kind) || (kind === 'receivable' && balance < 0)
}

/**
 * Form meminta utang sebagai angka positif; saldo liabilitas disimpan negatif. Piutang/hutang
 * memakai arah pilihan user: `iOwe` = user yang berutang (saldo negatif, D10).
 */
export const openingSign = (kind: AccountKindDto, iOwe = false) =>
  (isLiability(kind) || (kind === 'receivable' && iOwe) ? -1 : 1)

/**
 * Aset per jenis akun (urutan `ACCOUNT_KINDS`, jenis bernilai 0 dibuang) untuk komposisi kekayaan
 * di Dasbor. Saringan sama dengan `accounts_overview` + `net_worth` di Rust, jadi totalnya = `netWorth.assets`.
 */
export function assetsByKind(o: AccountsOverviewDto): { kind: AccountKindDto; amount: number }[] {
  const sums = new Map<AccountKindDto, number>()
  for (const a of o.accounts) {
    if (!a.includeInNetWorth || countsAsLiability(a.kind, a.balance)) continue
    sums.set(a.kind, (sums.get(a.kind) ?? 0) + a.balance)
  }
  return ACCOUNT_KINDS.filter((k) => (sums.get(k) ?? 0) !== 0).map((kind) => ({ kind, amount: sums.get(kind)! }))
}

/** Label arah saldo akun piutang/hutang untuk daftar akun. */
export function receivableLabel(balance: number): 'receivableLabel' | 'payableLabel' | 'settled' {
  return balance > 0 ? 'receivableLabel' : balance < 0 ? 'payableLabel' : 'settled'
}

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

/** Akun default form transaksi: terakhir dipakai bila masih aktif, else aset aktif pertama. */
export function defaultAccountId(accounts: AccountDto[], last: string | null): string | null {
  const active = accounts.filter((a) => !a.archived)
  if (last && active.some((a) => a.id === last)) return last
  return (active.find((a) => !isLiability(a.kind)) ?? active[0])?.id ?? null
}
