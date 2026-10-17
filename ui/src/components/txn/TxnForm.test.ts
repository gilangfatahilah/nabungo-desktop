import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it } from 'vitest'
import { nextTick } from 'vue'
import NewTxnSheet from '@/components/NewTxnSheet.vue'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import type { TxnFormDto } from '@/ipc'
import { formatDateInput, todayIso } from '@/lib/date'
import { useUiStore } from '@/stores/ui'

// Sheet di-teleport ke body: query lewat document.
const $ = <T extends Element = HTMLInputElement>(sel: string) => document.body.querySelector(sel) as T
let interactions = 0

async function settle() {
  await flushPromises()
  await nextTick()
  await flushPromises()
}

async function type(sel: string, value: string) {
  interactions++
  const el = $(sel)
  el.focus()
  el.value = value
  el.dispatchEvent(new Event('input', { bubbles: true }))
  await settle()
}

async function key(sel: string, k: string, opts: KeyboardEventInit = {}) {
  interactions++
  $(sel).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...opts }))
  await settle()
}

/** Ketik lalu Enter di combobox: satu interaksi (sesuai hitungan di plan). */
async function pick(sel: string, value: string) {
  await type(sel, value)
  await key(sel, 'Enter')
  interactions--
}

async function click(sel: string) {
  interactions++
  $<HTMLElement>(sel).click()
  await settle()
}

let bca = ''
const cats: Record<string, string> = {}

beforeAll(async () => {
  if (!(await ipc.getOnboardingStatus())) {
    bca = await ipc.completeOnboarding({
      locale: 'id', periodStartDay: 1, accountKind: 'bank',
      account: { name: 'BCA', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
        icon: null, color: null, openingBalance: 1_000_000, openingDate: '2026-10-01' },
    })
  }
  for (const c of await ipc.listCategories()) cats[c.name] = c.id
})

async function txns() {
  return ipc.listTxns({ dateFrom: null, dateTo: null, accountId: null, categoryId: null, tag: null, payeeId: null,
    amountMin: null, amountMax: null, search: null, limit: 1000, offset: 0 })
}

function expense(over: Partial<TxnFormDto> = {}): TxnFormDto {
  return { kind: 'expense', date: '2026-10-02', accountId: bca, toAccountId: null, amount: 25_000,
    splits: [{ categoryId: cats['Kopi & jajan']!, amount: 25_000, memo: null }], fee: null, payee: null,
    note: null, tags: [], ...over }
}

async function openSheet(action: (ui: ReturnType<typeof useUiStore>) => void) {
  mount(NewTxnSheet, { global: { plugins: [i18n] }, attachTo: document.body })
  const ui = useUiStore()
  action(ui)
  await settle()
  await settle()
  return ui
}

describe('TxnForm', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
    interactions = 0
  })
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('quick expense dalam lima interaksi', async () => {
    const before = (await txns()).total
    interactions++
    const ui = await openSheet((u) => u.openNewTxn())
    await type('#txn-amount', '25rb')
    await key('#txn-amount', 'Tab')
    await pick('#txn-category', 'Kopi')
    await key('#txn-category', 'Enter', { ctrlKey: true })
    // Tab di jsdom tidak memindah fokus; `type` memfokuskan field berikutnya tanpa interaksi tambahan.
    expect(interactions).toBe(5)
    const page = await txns()
    expect(page.total).toBe(before + 1)
    expect(page.rows[0]).toMatchObject({ kind: 'expense', amount: -25_000, categoryId: cats['Kopi & jajan'], accountId: bca })
    expect(ui.txnEditor).toBeNull()
  })

  it('split tidak cocok: error inline, sheet tetap terbuka', async () => {
    const before = (await txns()).total
    const ui = await openSheet((u) => u.openNewTxn())
    await type('#txn-amount', '75000')
    await click('[data-testid="split-toggle"]')
    await type('[data-testid="split-category-0"]', 'Kopi')
    await key('[data-testid="split-category-0"]', 'Enter')
    await type('[data-testid="split-amount-0"]', '50000')
    await type('[data-testid="split-category-1"]', 'Restoran')
    await key('[data-testid="split-category-1"]', 'Enter')
    await type('[data-testid="split-amount-1"]', '24999')
    await key('[data-testid="split-amount-1"]', 'Enter', { ctrlKey: true })
    expect($('[data-testid="split-remainder"]').textContent).toMatch(/Rp\s?1$/)
    expect($('[role="alert"]').textContent).toContain(id.errors.txn_split_mismatch)
    expect(ui.txnEditor).not.toBeNull()
    expect((await txns()).total).toBe(before)
  })

  it('transfer: biaya admin memakai kategori default', async () => {
    await openSheet((u) => u.openNewTxn())
    $<HTMLElement>('[data-testid="tab-transfer"]').dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }))
    await settle()
    await click('[data-testid="fee-toggle"]')
    expect($('#txn-fee-category').value).toBe('Biaya admin')
  })

  it('saran payee hanya mengisi kategori yang belum disentuh', async () => {
    await ipc.createTxn(expense({ payee: 'Starbucks' }))
    await openSheet((u) => u.openNewTxn())
    await type('#txn-payee', 'Starbucks')
    await key('#txn-payee', 'Enter')
    expect($('#txn-category').value).toBe('Makan & Minum › Kopi & jajan')
    document.body.innerHTML = ''

    setActivePinia(createPinia())
    await openSheet((u) => u.openNewTxn())
    await type('#txn-category', 'Restoran')
    await key('#txn-category', 'Enter')
    await type('#txn-payee', 'Starbucks')
    await key('#txn-payee', 'Enter')
    expect($('#txn-category').value).toBe('Makan & Minum › Restoran')
  })

  it('mode edit mengisi form dan memperbarui transaksi dengan id sama', async () => {
    const txnId = await ipc.createTxn(expense())
    const ui = await openSheet((u) => u.openEditTxn(txnId))
    expect($('#txn-amount').value).toBe('25000')
    expect($('#txn-category').value).toBe('Makan & Minum › Kopi & jajan')
    await type('#txn-amount', '30000')
    await key('#txn-amount', 'Enter', { ctrlKey: true })
    const d = await ipc.getTxn(txnId)
    expect(d.postings.map((p) => p.amount)).toEqual([-30_000])
    expect(ui.txnEditor).toBeNull()
  })

  it('duplikasi memakai tanggal hari ini dan membuat transaksi baru', async () => {
    const txnId = await ipc.createTxn(expense({ date: '2026-10-02', note: 'asli' }))
    const before = (await txns()).total
    await openSheet((u) => u.openDuplicateTxn(txnId))
    expect($('#txn-date').value).toBe(formatDateInput(todayIso(), 'id'))
    expect($<HTMLTextAreaElement>('#txn-note').value).toBe('asli')
    await key('#txn-amount', 'Enter', { ctrlKey: true })
    const page = await txns()
    expect(page.total).toBe(before + 1)
    expect(page.rows.some((r) => r.id !== txnId && r.note === 'asli' && r.date === todayIso())).toBe(true)
  })

  it('Tab dari nominal langsung ke kategori (urutan field sesuai brief)', async () => {
    await openSheet((u) => u.openNewTxn())
    const focusable = [...document.body.querySelectorAll<HTMLElement>('form input, form button, form textarea, form [tabindex]')]
      .filter((el) => !el.hasAttribute('disabled') && el.tabIndex >= 0 && !el.closest('[role="tablist"]'))
    const i = focusable.indexOf($('#txn-amount'))
    expect(i).toBeGreaterThanOrEqual(0)
    expect(focusable[i + 1]?.id).toBe('txn-category')
  })

  it('tag & catatan terlipat di transaksi baru dan terbuka lewat tombol', async () => {
    await openSheet((u) => u.openNewTxn())
    expect($('#txn-note')).toBeNull()
    await click('[data-testid="txn-more"]')
    expect($('#txn-note')).not.toBeNull()
    expect($('#txn-tags')).not.toBeNull()
  })

  it('nominal punya addon Rp dan tombol simpan di footer yang tidak ikut scroll', async () => {
    await openSheet((u) => u.openNewTxn())
    expect($<HTMLElement>('[data-testid="txn-amount-prefix"]').textContent).toBe('Rp')
    const footer = $<HTMLElement>('[data-testid="txn-form-footer"]')
    expect(footer.querySelector('button[type="submit"]')).not.toBeNull()
    expect(footer.closest('[data-testid="txn-form-body"]')).toBeNull()
  })
})
