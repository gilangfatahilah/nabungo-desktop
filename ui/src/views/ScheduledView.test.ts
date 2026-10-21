import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it } from 'vitest'
import { i18n } from '@/i18n'
import * as ipc from '@/ipc'
import type { TxnFormDto } from '@/ipc'
import { formatDateInput } from '@/lib/date'
import ScheduledView from './ScheduledView.vue'

const key = (el: Element, k: string) => el.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true }))
const button = (text: string) =>
  [...document.body.querySelectorAll('button, [role="menuitem"]')].find((b) => b.textContent?.trim() === text) as HTMLElement
const rows = (testid: string) => [...document.body.querySelectorAll<HTMLElement>(`[data-testid="${testid}"]`)]

const pad = (n: number) => String(n).padStart(2, '0')
const now = new Date()
// Tanggal 1 dua bulan lalu → 3 kejadian jatuh tempo (bulan ini termasuk).
const back = new Date(now.getFullYear(), now.getMonth() - 2, 1)
const start = `${back.getFullYear()}-${pad(back.getMonth() + 1)}-01`
const nextYear = `${now.getFullYear() + 1}-01-15`

let acc = ''
let cat = ''
const form = (date: string, payee: string): TxnFormDto => ({
  kind: 'expense', date, accountId: acc, toAccountId: null, amount: 150_000,
  splits: [{ categoryId: cat, amount: 150_000, memo: null }], fee: null, payee, note: null, tags: [],
})

beforeAll(async () => {
  acc = await ipc.createAccount('bank', { name: 'BCA SV', groupId: null, includeInNetWorth: true, creditLimit: null,
    statementDay: null, icon: null, color: null, openingBalance: 5_000_000, openingDate: '2020-01-01', contact: null })
  cat = await ipc.createCategory('expense', { name: 'Listrik SV', parentId: null, icon: null, color: null })
})

async function mountView() {
  mount(ScheduledView, { global: { plugins: [i18n] }, attachTo: document.body })
  await flushPromises()
  await flushPromises()
}

describe('ScheduledView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
  })
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('tanpa jadwal: empty state', async () => {
    await mountView()
    expect(document.body.textContent).toContain(i18n.global.t('scheduled.empty'))
    expect(rows('rule-row')).toHaveLength(0)
  })

  it('kejadian jatuh tempo berurutan; hanya yang tertua bisa ditangani', async () => {
    await ipc.createRule({ form: form(start, 'PLN'), recurrence: { freq: 'monthly', interval: 1, until: null }, autoPost: false })
    await mountView()
    const due = rows('due-row')
    expect(due).toHaveLength(3)
    expect(due[0]!.textContent).toContain(formatDateInput(start, 'id'))
    const confirms = rows('due-confirm')
    expect(confirms.map((b) => b.hasAttribute('disabled'))).toEqual([false, true, true])
    expect(rows('due-skip').map((b) => b.hasAttribute('disabled'))).toEqual([false, true, true])

    confirms[0]!.click()
    await flushPromises()
    await flushPromises()
    expect(rows('due-row')).toHaveLength(2)
  })

  it('konfirmasi semua dari menu menghabiskan kejadian jatuh tempo', async () => {
    await mountView()
    key(rows('due-menu')[0]!, 'Enter')
    await flushPromises()
    const all = document.body.querySelector<HTMLElement>('[data-testid="due-confirm-all"]')!
    expect(all.textContent?.trim()).toBe('Konfirmasi semua (2)')
    all.click()
    await flushPromises()
    await flushPromises()
    expect(rows('due-row')).toHaveLength(0)
    expect(document.body.textContent).not.toContain(i18n.global.t('scheduled.dueTitle'))
  })

  it('baris jadwal: deskripsi pengulangan dan badge Otomatis', async () => {
    await ipc.createRule({ form: form(nextYear, 'Netflix'), recurrence: { freq: 'monthly', interval: 1, until: null }, autoPost: true })
    await mountView()
    const row = rows('rule-row').find((r) => r.textContent?.includes('Netflix'))!
    expect(row.textContent).toContain('Tiap bulan, tgl 15')
    expect(row.textContent).toContain('Otomatis')
  })

  it('Hapus meminta konfirmasi lalu menghapus jadwal', async () => {
    const id = (await ipc.listRules()).find((r) => r.form.payee === 'Netflix')!.id
    await mountView()
    key(document.body.querySelector(`[data-testid="rule-menu-${id}"]`)!, 'Enter')
    await flushPromises()
    button('Hapus').click()
    await flushPromises()
    expect(document.body.querySelector('[role="alertdialog"]')).not.toBeNull()
    ;[...document.body.querySelectorAll<HTMLElement>('[role="alertdialog"] button')].find((b) => b.textContent?.trim() === 'Hapus')!.click()
    await flushPromises()
    await flushPromises()
    expect(rows('rule-row').some((r) => r.textContent?.includes('Netflix'))).toBe(false)
  })
})
