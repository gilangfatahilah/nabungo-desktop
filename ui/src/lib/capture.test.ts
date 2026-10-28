import { describe, expect, it } from 'vitest'
import type { AccountDto, CaptureDto, CategoryDto } from '@/ipc'
import { fromCapture, isReady } from './capture'

function account(id: string, archived = false): AccountDto {
  return {
    id, name: id, kind: 'bank', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
    icon: null, color: null, sortOrder: 0, archived, balance: 0, openingBalance: 0, openingDate: null,
    contact: null, reconciledThrough: null,
  }
}

function category(id: string, kind: 'income' | 'expense', archived = false): CategoryDto {
  return { id, parentId: null, kind, name: id, icon: null, color: null, sortOrder: 0, archived, inUse: false }
}

function capture(over: Partial<CaptureDto>): CaptureDto {
  return {
    id: 'cap-1', kind: 'expense', date: '2026-05-09', amount: 25_000, payee: 'Kopi Kenangan',
    accountId: 'bca', toAccountId: null, categoryId: 'makan', ruleName: 'BCA QRIS', snippet: null,
    duplicates: [], ...over,
  }
}

const ctx = {
  accounts: [account('bca'), account('jago'), account('lama', true)],
  categories: [category('makan', 'expense'), category('gaji', 'income'), category('kuno', 'expense', true)],
  feeCategoryId: 'biaya',
}

describe('capture', () => {
  it('expense_draft_maps_to_form', () => {
    const s = fromCapture(capture({}), ctx)
    expect(s).toMatchObject({
      kind: 'expense', date: '2026-05-09', amount: 25_000, accountId: 'bca', categoryId: 'makan',
      payee: 'Kopi Kenangan', split: false, feeEnabled: false, tags: [],
    })
    expect(s.note).toBe('')
  })

  it('transfer_draft_maps_to_and_fee_category', () => {
    const s = fromCapture(capture({ kind: 'transfer', toAccountId: 'jago', categoryId: null, payee: null }), ctx)
    expect(s).toMatchObject({ kind: 'transfer', accountId: 'bca', toAccountId: 'jago', feeCategoryId: 'biaya', payee: '' })
  })

  it('archived_draft_fields_are_dropped', () => {
    expect(fromCapture(capture({ accountId: 'lama' }), ctx).accountId).toBeNull()
    expect(fromCapture(capture({ categoryId: 'gaji' }), ctx).categoryId).toBeNull()
    expect(fromCapture(capture({ categoryId: 'kuno' }), ctx).categoryId).toBeNull()
    expect(fromCapture(capture({ kind: 'transfer', toAccountId: 'lama' }), ctx).toAccountId).toBeNull()
  })

  it('ready_rules', () => {
    expect(isReady(capture({}), ctx)).toBe(true)
    expect(isReady(capture({ kind: 'income', categoryId: 'gaji' }), ctx)).toBe(true)
    expect(isReady(capture({ accountId: null }), ctx)).toBe(false)
    expect(isReady(capture({ categoryId: null }), ctx)).toBe(false)
    expect(isReady(capture({ categoryId: 'gaji' }), ctx)).toBe(false)
    const dup = { id: 't1', date: '2026-05-09', payee: null, amount: 25_000, accountId: 'bca' }
    expect(isReady(capture({ duplicates: [dup] }), ctx)).toBe(false)
    const transfer = { kind: 'transfer' as const, categoryId: null }
    expect(isReady(capture({ ...transfer, toAccountId: 'jago' }), ctx)).toBe(true)
    expect(isReady(capture({ ...transfer, toAccountId: null }), ctx)).toBe(false)
    expect(isReady(capture({ ...transfer, toAccountId: 'bca' }), ctx)).toBe(false)
  })

  it('not_ready_when_account_archived', () => {
    expect(isReady(capture({ accountId: 'lama' }), ctx)).toBe(false)
    expect(isReady(capture({ kind: 'transfer', categoryId: null, toAccountId: 'lama' }), ctx)).toBe(false)
  })
})
