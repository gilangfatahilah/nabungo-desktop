import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import type { CaptureDto } from '@/ipc'
import { __seedEmailInbox } from '@/ipc/browser-ledger'
import CaptureSheet from './CaptureSheet.vue'

const toastMock = vi.hoisted(() => Object.assign(vi.fn(), { success: vi.fn(), error: vi.fn(), warning: vi.fn() }))
vi.mock('vue-sonner', () => ({ toast: toastMock }))

// Sheet di-teleport ke body: query lewat document.
const $ = <T extends Element = HTMLInputElement>(sel: string) => document.body.querySelector(sel) as T
const button = (text: string) =>
  [...document.body.querySelectorAll('button')].find((b) => b.textContent?.trim() === text) as HTMLButtonElement | undefined

async function settle() {
  await flushPromises()
  await nextTick()
  await flushPromises()
}

async function type(sel: string, value: string) {
  const el = $(sel)
  el.focus()
  el.value = value
  el.dispatchEvent(new Event('input', { bubbles: true }))
  await settle()
}

async function key(sel: string, k: string, opts: KeyboardEventInit = {}) {
  $(sel).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...opts }))
  await settle()
}

let acc = ''
let cat = ''

beforeAll(async () => {
  acc = await ipc.createAccount('bank', { name: 'BCA CS', groupId: null, includeInNetWorth: true, creditLimit: null,
    statementDay: null, icon: null, color: null, openingBalance: 1_000_000, openingDate: '2020-01-01', contact: null })
  cat = await ipc.createCategory('expense', { name: 'Makan CS', parentId: null, icon: null, color: null })
})

async function txnCount() {
  return (await ipc.listTxns({ dateFrom: null, dateTo: null, accountId: null, categoryId: null, tag: null, payeeId: null,
    amountMin: null, amountMax: null, search: null, sort: null, limit: 1, offset: 0 })).total
}

async function seeded(cid: string, over: { amount?: number; categoryId?: string | null; snippet?: string | null; date?: string } = {}) {
  __seedEmailInbox({ captures: [{ id: cid, kind: 'expense', date: '2026-05-09', amount: 41_700, payee: 'Indomaret',
    accountId: acc, toAccountId: null, categoryId: cat, ruleName: 'BCA QRIS', snippet: null, ...over }] })
  return (await ipc.listPendingCaptures()).find((c) => c.id === cid)!
}

async function openSheet(capture: CaptureDto) {
  const w = mount(CaptureSheet, { props: { capture: null }, global: { plugins: [i18n] }, attachTo: document.body })
  await w.setProps({ capture })
  await settle()
  await settle()
  return w
}

describe('CaptureSheet', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
    toastMock.success.mockClear()
  })
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('prefills_form_from_draft', async () => {
    await openSheet(await seeded('cs-1'))
    // Nominal terfokus (autofocus) tampil tanpa pemisah ribuan.
    expect($('#txn-amount').value).toMatch(/^41\.?700$/)
    expect($('#txn-payee').value).toBe('Indomaret')
    expect($<HTMLElement>('[data-testid="txn-account"]').textContent).toContain('BCA CS')
    expect($('#txn-category').value).toBe('Makan CS')
    expect($('#txn-date').value).toContain('9 Mei 2026')
  })

  it('save_confirms_capture', async () => {
    const before = await txnCount()
    const w = await openSheet(await seeded('cs-2', { categoryId: null, amount: 12_345 }))
    await type('#txn-category', 'Makan CS')
    await key('#txn-category', 'Enter')
    await key('#txn-category', 'Enter', { ctrlKey: true })
    expect(await txnCount()).toBe(before + 1)
    expect(w.emitted('close')).toHaveLength(1)
    expect(toastMock.success).toHaveBeenCalledWith(id.inbox.recorded)
    expect((await ipc.listPendingCaptures()).map((c) => c.id)).not.toContain('cs-2')
  })

  it('snippet_is_collapsed_by_default', async () => {
    await openSheet(await seeded('cs-3', { snippet: 'Isi email asli dari bank' }))
    const toggle = button(id.inbox.originalEmail)!
    expect(toggle.getAttribute('aria-expanded')).toBe('false')
    expect(document.body.textContent).not.toContain('Isi email asli dari bank')
    toggle.click()
    await settle()
    expect(document.body.textContent).toContain('Isi email asli dari bank')
  })

  it('shows_duplicates_with_link', async () => {
    await ipc.createTxn({ kind: 'expense', date: '2026-04-02', accountId: acc, toAccountId: null, amount: 77_000,
      splits: [{ categoryId: cat, amount: 77_000, memo: null }], fee: null, payee: 'Warung', note: null, tags: [] })
    const capture = await seeded('cs-4', { amount: 77_000, date: '2026-04-02' })
    expect(capture.duplicates).toHaveLength(1)
    const before = await txnCount()
    const w = await openSheet(capture)
    expect(document.body.textContent).toContain(id.inbox.duplicatesHint)
    expect($<HTMLElement>('[data-testid="duplicate-row"]').textContent).toContain('Warung')
    button(id.inbox.link)!.click()
    await settle()
    expect((await ipc.listPendingCaptures()).map((c) => c.id)).not.toContain('cs-4')
    expect(await txnCount()).toBe(before)
    expect(w.emitted('close')).toHaveLength(1)
    expect(toastMock.success).toHaveBeenCalledWith(id.inbox.linked)
  })

  it('not_pending_error_stays_open', async () => {
    const w = await openSheet(await seeded('cs-5'))
    await ipc.ignoreCapture('cs-5')
    await key('#txn-amount', 'Enter', { ctrlKey: true })
    expect($<HTMLElement>('[role="alert"]').textContent).toBe(id.errors.capture_not_pending)
    expect(w.emitted('close')).toBeUndefined()
    expect($('#txn-amount')).not.toBeNull()
  })
})
