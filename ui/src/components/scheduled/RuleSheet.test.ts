import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { i18n } from '@/i18n'
import * as ipc from '@/ipc'
import type { TxnFormDto } from '@/ipc'
import { formatDateInput } from '@/lib/date'
import { useUiStore } from '@/stores/ui'
import RuleSheet from './RuleSheet.vue'

const $ = <T extends Element = HTMLInputElement>(sel: string) => document.body.querySelector(sel) as T

async function settle() {
  await flushPromises()
  await nextTick()
  await flushPromises()
}

async function save() {
  $('#txn-amount').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true, bubbles: true, cancelable: true }))
  await settle()
  await settle()
}

let acc = ''
let cat = ''

const expense = (date: string, amount = 40_000): TxnFormDto => ({
  kind: 'expense', date, accountId: acc, toAccountId: null, amount, splits: [{ categoryId: cat, amount, memo: null }],
  fee: null, payee: 'PLN RS', note: null, tags: [],
})

beforeAll(async () => {
  acc = await ipc.createAccount('bank', { name: 'Jadwal RS', groupId: null, includeInNetWorth: true, creditLimit: null,
    statementDay: null, icon: null, color: null, openingBalance: 0, openingDate: '2026-01-01', contact: null })
  cat = await ipc.createCategory('expense', { name: 'Listrik RS', parentId: null, icon: null, color: null })
})

async function openSheet(action: (ui: ReturnType<typeof useUiStore>) => void) {
  mount(RuleSheet, { global: { plugins: [i18n] }, attachTo: document.body })
  const ui = useUiStore()
  action(ui)
  await settle()
  await settle()
  return ui
}

describe('RuleSheet', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
  })
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('mode create: field pengulangan tampil dan simpan membuat jadwal', async () => {
    const before = (await ipc.listRules()).length
    const ui = await openSheet((u) => u.openRuleEditor({ mode: 'create' }))
    expect($('[data-testid="recurrence-fields"]')).not.toBeNull()
    expect(document.body.textContent).toContain('Jadwal baru')
    const amount = $('#txn-amount')
    amount.value = '15000'
    amount.dispatchEvent(new Event('input', { bubbles: true }))
    await settle()
    const c = $('#txn-category')
    c.focus()
    c.value = 'Listrik RS'
    c.dispatchEvent(new Event('input', { bubbles: true }))
    await settle()
    c.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }))
    await settle()
    await save()
    const rules = await ipc.listRules()
    expect(rules.length).toBe(before + 1)
    expect(rules.some((r) => r.form.amount === 15_000)).toBe(true)
    expect(ui.ruleEditor).toBeNull()
  })

  it('mode fromTxn: terisi dari transaksi dan tanggal = kejadian kedua; pratinjau 3 tanggal', async () => {
    const { id } = await ipc.createTxn(expense('2026-01-31'))
    await openSheet((u) => u.openRuleEditor({ mode: 'fromTxn', txnId: id }))
    expect(document.body.textContent).toContain('Jadikan berulang')
    expect($('#txn-amount').value).toBe('40000')
    expect($('#txn-date').value).toBe(formatDateInput('2026-02-28', 'id'))
    await vi.waitFor(() => {
      const items = document.body.querySelectorAll('[data-testid="recurrence-preview"] li')
      expect([...items].map((x) => x.textContent)).toEqual(['2026-02-28', '2026-03-28', '2026-04-28'].map((d) => formatDateInput(d, 'id')))
    })
  })

  it('mode occurrence: tanpa field pengulangan; simpan mencatat kejadian', async () => {
    const ruleId = await ipc.createRule({ form: expense('2026-10-05', 33_000), recurrence: { freq: 'monthly', interval: 1, until: null }, autoPost: false })
    const ui = await openSheet((u) => u.openRuleEditor({ mode: 'occurrence', ruleId, date: '2026-10-05' }))
    expect(document.body.textContent).toContain('Catat kejadian')
    expect($('[data-testid="recurrence-fields"]')).toBeNull()
    expect($('#txn-amount').value).toBe('33000')
    await save()
    const rule = (await ipc.listRules()).find((r) => r.id === ruleId)!
    expect(rule.form.date).toBe('2026-11-05')
    expect(ui.ruleEditor).toBeNull()
  })
})
