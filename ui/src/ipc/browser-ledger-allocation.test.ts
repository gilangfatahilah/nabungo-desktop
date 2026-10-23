import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { browserLedger as Ledger } from './browser-ledger'

// Kasus yang sama dengan test `allocation.rs` di core; tiap test memakai state browser-ledger baru
// agar gross (total aset) terkendali.
let L: typeof Ledger

beforeEach(async () => {
  vi.resetModules()
  L = (await import('./browser-ledger')).browserLedger
})

async function data<T>(p: Promise<{ status: 'ok'; data: T } | { status: 'error'; error: unknown }>): Promise<T> {
  const r = await p
  if (r.status !== 'ok') throw new Error(JSON.stringify(r.error))
  return r.data
}

const bank = (opening: number) => data(L.createAccount('bank', {
  name: 'Bank', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
  icon: null, color: null, openingBalance: opening, openingDate: '2026-10-01', contact: null,
}))
const percent = (name: string, parentId: string | null, targetBp: number) =>
  data(L.createBucket({ name, parentId, targetBp, targetAmount: null, color: null }))
const nominal = (name: string, parentId: string | null, targetAmount: number) =>
  data(L.createBucket({ name, parentId, targetBp: 0, targetAmount, color: null }))

async function targets() {
  const o = await data(L.listAllocation())
  const t = (id: string) => {
    const a = o.actuals.find((x) => x.bucketId === id)!
    return [a.targetAmount, a.targetBp]
  }
  return { o, t }
}

describe('browser ledger target nominal (paritas Rust)', () => {
  it('nominal bucket takes amount first', async () => {
    await bank(50_000_000)
    const darurat = await nominal('Darurat', null, 30_000_000)
    const invest = await percent('Investasi', null, 6000)
    const liquid = await percent('Liquid', null, 4000)
    const { o, t } = await targets()
    expect(t(darurat)).toEqual([30_000_000, 6000])
    expect(t(invest)).toEqual([12_000_000, 2400])
    expect(t(liquid)).toEqual([8_000_000, 1600])
    expect(o.buckets[0]).toMatchObject({ targetBp: 0, targetAmount: 30_000_000 })
    expect(o.warnings).toEqual([])
    expect(o.amountWarnings).toEqual([])
  })

  it('child percent of nominal parent', async () => {
    await bank(60_000_000)
    const darurat = await nominal('Darurat', null, 30_000_000)
    const a = await percent('A', darurat, 5000)
    expect((await targets()).t(a)).toEqual([15_000_000, 2500])
  })

  it('nominal exceeding base warns', async () => {
    const acc = await bank(50_000_000)
    const x = await nominal('X', null, 30_000_000)
    await nominal('Y', null, 30_000_000)
    const z = await percent('Z', null, 10000)
    await data(L.setAccountAllocation(acc, [{ bucketId: x, shareBp: 10000 }]))
    const { o, t } = await targets()
    expect(o.amountWarnings).toEqual([{ parentId: null, totalAmount: 60_000_000, base: 50_000_000 }])
    expect(t(z)).toEqual([0, 0])
    expect(o.actuals.find((b) => b.bucketId === x)!.diffAmount).toBe(20_000_000)
    expect(o.warnings).toEqual([])
  })

  it('zero gross with nominal', async () => {
    const x = await nominal('X', null, 1_000_000)
    const z = await percent('Z', null, 10000)
    const { o, t } = await targets()
    expect(t(x)).toEqual([1_000_000, 0])
    expect(t(z)).toEqual([0, 10000])
    expect(o.amountWarnings).toEqual([])
  })

  it('percent warning ignores nominal', async () => {
    await nominal('X', null, 1_000)
    const p = await percent('P', null, 10000)
    await nominal('C', p, 500)
    expect((await targets()).o.warnings).toEqual([])
  })

  it('percent only matches fase 5 rule', async () => {
    await bank(10_000_000)
    const aset = await percent('Aset', null, 7000)
    const saham = await percent('Saham', aset, 6000)
    const emas = await percent('Emas', aset, 4000)
    const liquid = await percent('Liquid', null, 3000)
    const { t } = await targets()
    expect([aset, saham, emas, liquid].map((id) => t(id)[1])).toEqual([7000, 4200, 2800, 3000])
    expect(t(saham)[0]).toBe(4_200_000)
  })

  it('validates amount and switches kind', async () => {
    expect(await L.createBucket({ name: 'D', parentId: null, targetBp: 0, targetAmount: 0, color: null }))
      .toMatchObject({ status: 'error', error: { code: 'allocation_invalid_amount' } })
    const a = await percent('A', null, 6000)
    await percent('B', null, 4000)
    await data(L.updateBucket(a, { name: 'A', parentId: null, targetBp: 6000, targetAmount: 1_000, color: null }))
    expect((await data(L.listAllocation())).buckets[0]).toMatchObject({ targetBp: 0, targetAmount: 1_000 })
    await percent('C', null, 6000)
    expect(await L.updateBucket(a, { name: 'A', parentId: null, targetBp: 1, targetAmount: null, color: null }))
      .toMatchObject({ status: 'error', error: { code: 'allocation_targets_exceed' } })
  })

  it('child diff amount uses parent amount', async () => {
    // Paritas `child_diff_amount_uses_parent_amount`: floor(floor(gross × p) × c), bukan gross × floor(p × c).
    await bank(1_000_000)
    const root = await percent('Root', null, 3333)
    const child = await percent('Child', root, 3333)
    const { o, t } = await targets()
    expect(t(child)).toEqual([111_088, 1110])
    expect(o.actuals.find((x) => x.bucketId === child)!.diffAmount).toBe(-111_088)
  })
})
