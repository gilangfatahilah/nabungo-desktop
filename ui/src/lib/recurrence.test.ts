import { describe, expect, it } from 'vitest'
import { i18n } from '@/i18n'
import type { RuleDto, TxnFormDto } from '@/ipc'
import { describeRecurrence, ruleName } from './recurrence'

const t = i18n.global.t as unknown as Parameters<typeof describeRecurrence>[2]

const form = (over: Partial<TxnFormDto> = {}): TxnFormDto => ({
  kind: 'expense', date: '2026-10-25', accountId: 'a', toAccountId: null, amount: 1, splits: [{ categoryId: 'c', amount: 1, memo: null }],
  fee: null, payee: null, note: null, tags: [], ...over,
})
const rule = (f: TxnFormDto): RuleDto => ({ id: 'r', form: f, recurrence: { freq: 'monthly', interval: 1, until: null }, autoPost: false, ended: false, due: [] })

describe('describeRecurrence', () => {
  it('bulanan, mingguan, tahunan, harian', () => {
    i18n.global.locale.value = 'id'
    expect(describeRecurrence({ freq: 'monthly', interval: 1, until: null }, '2026-10-25', t, 'id')).toBe('Tiap bulan, tgl 25')
    expect(describeRecurrence({ freq: 'weekly', interval: 2, until: null }, '2026-10-19', t, 'id')).toBe('Tiap 2 minggu, Senin')
    expect(describeRecurrence({ freq: 'yearly', interval: 1, until: null }, '2028-02-29', t, 'id')).toBe('Tiap tahun, 29 Feb')
    expect(describeRecurrence({ freq: 'daily', interval: 1, until: null }, '2026-10-25', t, 'id')).toBe('Tiap hari')
  })
})

describe('ruleName', () => {
  it('payee > kategori > catatan > label jenis', () => {
    i18n.global.locale.value = 'id'
    const cat = (id: string) => (id === 'c' ? 'Listrik' : undefined)
    expect(ruleName(rule(form({ payee: 'PLN', note: 'token' })), cat, t)).toBe('PLN')
    expect(ruleName(rule(form({ note: 'token' })), cat, t)).toBe('Listrik')
    expect(ruleName(rule(form({ splits: [], note: 'token' })), cat, t)).toBe('token')
    expect(ruleName(rule(form({ kind: 'transfer', splits: [] })), cat, t)).toBe('Transfer')
  })
})
