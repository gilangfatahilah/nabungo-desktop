import { flushPromises, mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import { __seedEmailInbox } from '@/ipc/browser-ledger'
import { routes } from '@/router'
import { useInboxStore } from '@/stores/inbox'
import { useTransactionsStore } from '@/stores/transactions'
import InboxView from './InboxView.vue'

const toastMock = vi.hoisted(() => Object.assign(vi.fn(), { success: vi.fn(), error: vi.fn(), warning: vi.fn() }))
vi.mock('vue-sonner', () => ({ toast: toastMock }))

const key = (el: Element, k: string) => el.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true }))
const button = (text: string) =>
  [...document.body.querySelectorAll('button, [role="menuitem"]')].find((b) => b.textContent?.trim() === text) as HTMLElement
const $ = (sel: string) => document.body.querySelector(sel) as HTMLElement | null
const row = (captureId: string) => $(`[data-testid="capture-row"][data-capture-id="${captureId}"]`)

let acc = ''
let cat = ''

const seed = (cid: string, over: { date?: string; payee?: string | null; categoryId?: string | null; amount?: number } = {}) => ({
  id: cid, kind: 'expense' as const, date: '2026-05-09', amount: 41_700, payee: `Toko ${cid}`, accountId: acc,
  toAccountId: null, categoryId: cat, ruleName: 'BCA QRIS', snippet: null, ...over,
})

beforeAll(async () => {
  acc = await ipc.createAccount('bank', { name: 'BCA IV', groupId: null, includeInNetWorth: true, creditLimit: null,
    statementDay: null, icon: null, color: null, openingBalance: 1_000_000, openingDate: '2020-01-01', contact: null })
  cat = await ipc.createCategory('expense', { name: 'Makan IV', parentId: null, icon: null, color: null })
})

async function mountView() {
  const router = createRouter({ history: createMemoryHistory(), routes })
  await router.push('/inbox')
  const w = mount(InboxView, { global: { plugins: [i18n, router] }, attachTo: document.body })
  await flushPromises()
  await flushPromises()
  return { w, router }
}

async function openMenu(captureId: string) {
  key(row(captureId)!.querySelector('[data-testid="capture-menu"]')!, 'Enter')
  await flushPromises()
}

describe('InboxView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
    toastMock.mockClear()
    toastMock.success.mockClear()
  })
  afterEach(() => {
    vi.restoreAllMocks()
    document.body.innerHTML = ''
  })

  it('lists_pending_grouped_by_date', async () => {
    __seedEmailInbox({ captures: [seed('g-1', { date: '2026-05-08' }), seed('g-2'), seed('g-3')] })
    await mountView()
    const headers = [...document.body.querySelectorAll('[data-testid="inbox-date"]')].map((h) => h.getAttribute('data-date'))
    expect(headers).toEqual(expect.arrayContaining(['2026-05-09', '2026-05-08']))
    expect(headers.filter((d) => d === '2026-05-09')).toHaveLength(1)
    const r = row('g-2')!
    expect(r.textContent).toContain('Toko g-2')
    expect(r.textContent).toContain('BCA QRIS')
    expect(r.textContent).toContain('BCA IV')
    expect(r.textContent).toContain('Makan IV')
  })

  it('quick_confirm_only_when_ready', async () => {
    __seedEmailInbox({ captures: [seed('r-1'), seed('r-2', { categoryId: null, payee: null })] })
    await mountView()
    expect(row('r-1')!.querySelector('[data-testid="capture-record"]')).not.toBeNull()
    const notReady = row('r-2')!
    expect(notReady.querySelector('[data-testid="capture-record"]')).toBeNull()
    expect(notReady.textContent).toContain(id.inbox.pickCategory)
    // Payee kosong → nama rule di baris 1.
    expect(notReady.querySelector('[data-part="title"]')!.textContent).toBe('BCA QRIS')
    expect(notReady.textContent).toContain(id.inbox.review)
  })

  it('quick_confirm_records_and_removes_row', async () => {
    __seedEmailInbox({ captures: [seed('q-1')] })
    await mountView()
    ;(row('q-1')!.querySelector('[data-testid="capture-record"]') as HTMLElement).click()
    await flushPromises()
    await flushPromises()
    expect(row('q-1')).toBeNull()
    expect(toastMock.success).toHaveBeenCalledWith(id.inbox.recorded)
  })

  it('ignore_with_undo', async () => {
    __seedEmailInbox({ captures: [seed('i-1')] })
    await mountView()
    await openMenu('i-1')
    button(id.inbox.ignore).click()
    await flushPromises()
    expect(row('i-1')).toBeNull()
    const [msg, opts] = toastMock.mock.calls.at(-1)! as [string, { action: { label: string; onClick: () => void } }]
    expect(msg).toBe(id.inbox.ignored)
    expect(opts.action.label).toBe(id.inbox.undo)
    opts.action.onClick()
    await flushPromises()
    await flushPromises()
    expect(row('i-1')).not.toBeNull()
  })

  it('ignore_until_dialog_previews_count_and_ignores', async () => {
    __seedEmailInbox({ captures: [seed('u-1', { date: '2025-01-02' }), seed('u-2', { date: '2025-01-03' }), seed('u-3', { date: '2025-01-20' })] })
    await mountView()
    key($('[data-testid="inbox-menu"]')!, 'Enter')
    await flushPromises()
    button(id.inbox.ignoreUntil).click()
    await flushPromises()
    const input = $('#ignore-until-date') as HTMLInputElement
    input.value = '2025-01-10'
    input.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()
    key(input, 'Enter')
    await flushPromises()
    expect($('[data-testid="ignore-until-hint"]')!.textContent).toContain('2 item')
    ;($('[data-testid="ignore-until-submit"]') as HTMLElement).click()
    await flushPromises()
    await flushPromises()
    expect(row('u-1')).toBeNull()
    expect(row('u-2')).toBeNull()
    expect(row('u-3')).not.toBeNull()
    const [msg, opts] = toastMock.mock.calls.at(-1)! as [string, { action: { onClick: () => void } }]
    expect(msg).toBe('2 email diabaikan')
    opts.action.onClick()
    await flushPromises()
    await flushPromises()
    expect(row('u-1')).not.toBeNull()
  })

  it('unparsed_tab_shows_subject_and_toggles_snippet', async () => {
    __seedEmailInbox({
      unparsed: [
        { messageId: '<a@x>', sender: 'bank@x', subject: 'Promo akhir tahun', receivedAt: '2026-05-09T03:00:00Z', snippet: 'Isi promo' },
        { messageId: '<b@x>', sender: 'bank@x', subject: '', receivedAt: null, snippet: 'Tanpa subjek' },
      ],
    })
    const { w } = await mountView()
    const tab = w.findAll('[role="tab"]').find((t) => t.text().startsWith(id.inbox.tabUnparsed))!
    await tab.trigger('mousedown', { button: 0 })
    await flushPromises()
    const items = [...document.body.querySelectorAll<HTMLElement>('[data-testid="unparsed-row"]')]
    const a = items.find((i) => i.dataset.messageId === '<a@x>')!
    const b = items.find((i) => i.dataset.messageId === '<b@x>')!
    expect(a.textContent).toContain('Promo akhir tahun')
    expect(b.textContent).toContain(id.inbox.noSubject)
    expect(a.textContent).not.toContain('Isi promo')
    const toggle = a.querySelector('button')!
    expect(toggle.getAttribute('aria-expanded')).toBe('false')
    toggle.click()
    await flushPromises()
    expect(toggle.getAttribute('aria-expanded')).toBe('true')
    expect(a.textContent).toContain('Isi promo')
  })

  it('not_configured_empty_state_links_settings', async () => {
    vi.spyOn(ipc, 'inboxSummary').mockResolvedValue({ pending: 0, unparsed: 0, emailEnabled: false })
    vi.spyOn(ipc, 'listPendingCaptures').mockResolvedValue([])
    vi.spyOn(ipc, 'listUnparsedEmails').mockResolvedValue([])
    await mountView()
    expect(document.body.textContent).toContain(id.inbox.notConfiguredTitle)
    const link = [...document.body.querySelectorAll('a')].find((a) => a.textContent?.trim() === id.inbox.notConfiguredAction)!
    expect(link.getAttribute('href')).toContain('/settings')
    expect($('[role="tablist"]')).toBeNull()
  })

  it('reload_keeps_open_sheet', async () => {
    __seedEmailInbox({ captures: [seed('k-1', { amount: 5_555 })] })
    await mountView()
    ;(row('k-1')!.querySelector('[data-testid="capture-review"]') as HTMLElement).click()
    await flushPromises()
    await flushPromises()
    const payee = $('#txn-payee') as HTMLInputElement
    payee.value = 'Edit baru'
    payee.dispatchEvent(new Event('input', { bubbles: true }))
    await flushPromises()
    await useInboxStore().reload()
    await flushPromises()
    expect(($('#txn-payee') as HTMLInputElement).value).toBe('Edit baru')
    expect(document.body.textContent).toContain(id.inbox.reviewTitle)
  })

  it('duplicate_badge_opens_dialog', async () => {
    await ipc.createTxn({ kind: 'expense', date: '2026-03-03', accountId: acc, toAccountId: null, amount: 66_000,
      splits: [{ categoryId: cat, amount: 66_000, memo: null }], fee: null, payee: 'Warung IV', note: null, tags: [] })
    __seedEmailInbox({ captures: [seed('d-1', { amount: 66_000, date: '2026-03-03' })] })
    await mountView()
    expect(row('d-1')!.querySelector('[data-testid="capture-record"]')).toBeNull()
    ;(row('d-1')!.querySelector('[data-testid="capture-duplicates"]') as HTMLElement).click()
    await flushPromises()
    expect($('[role="dialog"]')!.textContent).toContain(id.inbox.duplicatesTitle)
    expect($('[data-testid="duplicate-row"]')!.textContent).toContain('Warung IV')
  })

  it('manual_txn_elsewhere_reloads_list', async () => {
    __seedEmailInbox({ captures: [seed('m-1', { amount: 88_000, date: '2026-02-02' })] })
    await mountView()
    expect(row('m-1')!.querySelector('[data-testid="capture-record"]')).not.toBeNull()
    // Dicatat lewat sheet global (N) saat Inbox terbuka.
    await ipc.createTxn({ kind: 'expense', date: '2026-02-02', accountId: acc, toAccountId: null, amount: 88_000,
      splits: [{ categoryId: cat, amount: 88_000, memo: null }], fee: null, payee: 'Manual', note: null, tags: [] })
    await useTransactionsStore().refresh()
    await flushPromises()
    await flushPromises()
    expect(row('m-1')!.querySelector('[data-testid="capture-record"]')).toBeNull()
    expect(row('m-1')!.querySelector('[data-testid="capture-duplicates"]')).not.toBeNull()
  })
})
