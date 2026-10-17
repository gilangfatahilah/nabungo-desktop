import { describe, expect, it } from 'vitest'
import type { AccountDto, AccountsOverviewDto } from '@/ipc'
import { groupAccounts } from './accounts'

function acc(id: string, groupId: string | null, balance: number, extra: Partial<AccountDto> = {}): AccountDto {
  return {
    id, name: id, kind: 'bank', groupId, includeInNetWorth: true, creditLimit: null, statementDay: null,
    icon: null, color: null, sortOrder: 0, archived: false, balance, openingBalance: 0, openingDate: null, ...extra,
  }
}

const overview: AccountsOverviewDto = {
  groups: [
    { id: 'g2', name: 'Tabungan', sortOrder: 2 },
    { id: 'g1', name: 'Harian', sortOrder: 1 },
    { id: 'g3', name: 'Kosong', sortOrder: 3 },
  ],
  accounts: [
    acc('bca', 'g1', 1_000_000),
    acc('cc', 'g1', -250_000, { kind: 'credit_card' }),
    acc('dana', 'g2', 50_000),
    acc('dompet', null, 20_000),
    acc('lama', 'g1', 5_000, { archived: true }),
  ],
  netWorth: { assets: 0, liabilities: 0, net: 0 },
}

describe('groupAccounts', () => {
  it('urut group, tanpa-grup terakhir, arsip disembunyikan, subtotal integer', () => {
    const s = groupAccounts(overview, { archived: false })
    expect(s.map((x) => x.group?.id ?? null)).toEqual(['g1', 'g2', 'g3', null])
    expect(s[0]!.accounts.map((a) => a.id)).toEqual(['bca', 'cc'])
    expect(s[0]!.subtotal).toBe(750_000)
    expect(s[2]!.accounts).toEqual([])
    expect(s[2]!.subtotal).toBe(0)
    expect(s[3]!.subtotal).toBe(20_000)
  })

  it('tanpa akun tanpa-grup → tidak ada section null', () => {
    const o = { ...overview, accounts: overview.accounts.filter((a) => a.groupId) }
    expect(groupAccounts(o, { archived: false }).some((x) => x.group === null)).toBe(false)
  })

  it('archived: true → satu section berisi akun diarsip', () => {
    const s = groupAccounts(overview, { archived: true })
    expect(s).toHaveLength(1)
    expect(s[0]!.group).toBeNull()
    expect(s[0]!.accounts.map((a) => a.id)).toEqual(['lama'])
    expect(s[0]!.subtotal).toBe(5_000)
  })
})
