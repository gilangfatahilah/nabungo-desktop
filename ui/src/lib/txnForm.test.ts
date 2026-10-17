import { describe, expect, it } from 'vitest'
import type { TxnDetailDto } from '@/ipc'
import { duplicateOf, emptyForm, fromDetail, isEditable, splitRemainder, toDto } from './txnForm'

const base = { accountId: 'acc-a', feeCategoryId: 'cat-fee', today: '2026-10-17' }

function detail(over: Partial<TxnDetailDto>): TxnDetailDto {
  return { id: 't1', kind: 'expense', date: '2026-10-03', payee: null, note: null, tags: [], postings: [], ...over }
}

describe('txnForm', () => {
  it('emptyForm memakai default akun, kategori biaya, dan hari ini', () => {
    const s = emptyForm('transfer', base)
    expect(s).toMatchObject({ kind: 'transfer', date: '2026-10-17', accountId: 'acc-a', feeCategoryId: 'cat-fee', feeEnabled: false })
  })

  it('toDto expense non-split → satu split', () => {
    const s = { ...emptyForm('expense', base), amount: 25_000, categoryId: 'cat-kopi', payee: ' Kopi ', note: '', tags: ['kantor'] }
    expect(toDto(s)).toEqual({
      kind: 'expense', date: '2026-10-17', accountId: 'acc-a', toAccountId: null, amount: 25_000,
      splits: [{ categoryId: 'cat-kopi', amount: 25_000, memo: null }], fee: null,
      payee: 'Kopi', note: null, tags: ['kantor'],
    })
  })

  it('toDto split memakai baris split; kategori kosong → splits kosong (missing category)', () => {
    const s = {
      ...emptyForm('expense', base), amount: 50_000, split: true,
      splits: [{ categoryId: 'a', amount: 30_000, memo: ' m ' }, { categoryId: 'b', amount: 20_000, memo: '' }],
    }
    expect(toDto(s).splits).toEqual([
      { categoryId: 'a', amount: 30_000, memo: 'm' },
      { categoryId: 'b', amount: 20_000, memo: null },
    ])
    expect(toDto({ ...s, splits: [{ categoryId: null, amount: 50_000, memo: '' }] }).splits).toEqual([])
    expect(toDto({ ...emptyForm('expense', base), amount: 1 }).splits).toEqual([])
  })

  it('toDto transfer dengan biaya', () => {
    const s = { ...emptyForm('transfer', base), toAccountId: 'acc-b', amount: 200_000, feeEnabled: true, fee: 6_500 }
    expect(toDto(s)).toMatchObject({ kind: 'transfer', toAccountId: 'acc-b', splits: [], fee: { amount: 6_500, categoryId: 'cat-fee' } })
    expect(toDto({ ...s, feeEnabled: false }).fee).toBeNull()
    expect(toDto({ ...s, feeCategoryId: null }).fee).toEqual({ amount: 6_500, categoryId: null })
  })

  it('fromDetail transfer dengan biaya', () => {
    const s = fromDetail(detail({
      kind: 'transfer',
      postings: [
        { accountId: 'acc-a', categoryId: null, amount: -200_000, memo: null },
        { accountId: 'acc-b', categoryId: null, amount: 200_000, memo: null },
        { accountId: 'acc-a', categoryId: 'cat-fee', amount: -6_500, memo: null },
      ],
    }))
    expect(s).toMatchObject({
      kind: 'transfer', accountId: 'acc-a', toAccountId: 'acc-b', amount: 200_000,
      feeEnabled: true, fee: 6_500, feeCategoryId: 'cat-fee',
    })
  })

  it('fromDetail refund expense → nominal negatif', () => {
    const s = fromDetail(detail({ postings: [{ accountId: 'acc-a', categoryId: 'c', amount: 20_000, memo: null }] }))
    expect(s).toMatchObject({ kind: 'expense', amount: -20_000, categoryId: 'c', split: false })
  })

  it('fromDetail split expense dan income', () => {
    const s = fromDetail(detail({
      payee: 'Indomaret', tags: ['x'],
      postings: [
        { accountId: 'acc-a', categoryId: 'a', amount: -30_000, memo: 'm' },
        { accountId: 'acc-a', categoryId: 'b', amount: -20_000, memo: null },
      ],
    }))
    expect(s).toMatchObject({
      amount: 50_000, split: true, payee: 'Indomaret', tags: ['x'],
      splits: [{ categoryId: 'a', amount: 30_000, memo: 'm' }, { categoryId: 'b', amount: 20_000, memo: '' }],
    })
    const inc = fromDetail(detail({ kind: 'income', postings: [{ accountId: 'acc-a', categoryId: 'g', amount: 5_000, memo: null }] }))
    expect(inc).toMatchObject({ kind: 'income', amount: 5_000, categoryId: 'g' })
  })

  it('duplicateOf mengganti tanggal', () => {
    const d = detail({ postings: [{ accountId: 'acc-a', categoryId: 'c', amount: -1, memo: null }] })
    expect(duplicateOf(d, '2026-10-17')).toMatchObject({ date: '2026-10-17', amount: 1, categoryId: 'c' })
  })

  it('splitRemainder = nominal − Σ split', () => {
    const s = { ...emptyForm('expense', base), amount: 50_000, split: true, splits: [{ categoryId: 'a', amount: 30_000, memo: '' }] }
    expect(splitRemainder(s)).toBe(20_000)
  })

  it('isEditable hanya income/expense/transfer', () => {
    expect(isEditable('expense')).toBe(true)
    expect(isEditable('transfer')).toBe(true)
    expect(isEditable('opening')).toBe(false)
    expect(isEditable('adjustment')).toBe(false)
  })
})
