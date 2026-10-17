import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { useAccountsStore } from './accounts'

describe('accounts store (fallback browser)', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('aksi memuat ulang overview setelah sukses', async () => {
    const s = useAccountsStore()
    await s.createAccountGroup('Harian')
    const group = s.overview.groups.find((g) => g.name === 'Harian')
    expect(group).toBeTruthy()
    await s.createAccount('bank', {
      name: 'BCA', groupId: group!.id, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 1_000_000, openingDate: '2026-10-17',
    })
    const bca = s.overview.accounts.find((a) => a.name === 'BCA')
    expect(bca?.balance).toBe(1_000_000)
    expect(bca?.groupId).toBe(group!.id)
    expect(s.overview.netWorth.net).toBeGreaterThanOrEqual(1_000_000)
  })
})

describe('net worth fallback browser', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('akun diarsip tetap dihitung, seperti Rust', async () => {
    const s = useAccountsStore()
    const id = await s.createAccount('cash', {
      name: 'Celengan', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 250_000, openingDate: '2026-10-17',
    })
    const before = s.overview.netWorth.net
    await s.archiveAccount(id)
    expect(s.overview.netWorth.net).toBe(before)
  })
})
