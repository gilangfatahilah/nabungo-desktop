import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import type { EmailRuleDto, EmailRuleInputDto } from '@/ipc'
import { __seedEmailInbox } from '@/ipc/browser-ledger'
import { useInboxStore } from '@/stores/inbox'
import EmailRuleSheet from './EmailRuleSheet.vue'
import { pickOption } from '@/test/select'

const toastMock = vi.hoisted(() => Object.assign(vi.fn(), { success: vi.fn(), error: vi.fn(), warning: vi.fn() }))
vi.mock('vue-sonner', () => ({ toast: toastMock }))

// Sheet di-teleport ke body: query lewat document.
const $ = <T extends Element = HTMLInputElement>(sel: string) => document.body.querySelector(sel) as T
const button = (text: string) =>
  [...document.body.querySelectorAll('button, [role="tab"]')].find((b) => b.textContent?.trim() === text) as HTMLElement | undefined
const key = (el: Element, k: string) => el.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true }))

async function settle() {
  await flushPromises()
  await nextTick()
  await flushPromises()
}

async function type(sel: string, value: string) {
  const el = $(sel)
  el.value = value
  el.dispatchEvent(new Event('input', { bubbles: true }))
  await settle()
}

async function pick(testid: string, label: string) {
  key($(`[data-testid="${testid}"]`), 'Enter')
  await settle()
  const opt = [...document.body.querySelectorAll('[role="option"]')].find((o) => o.textContent?.trim() === label)!
  key(opt, 'Enter')
  await settle()
}

let acc = ''
let acc2 = ''
let catExpense = ''

beforeAll(async () => {
  const account = (name: string) => ipc.createAccount('bank', { name, groupId: null, includeInNetWorth: true, creditLimit: null,
    statementDay: null, icon: null, color: null, openingBalance: 0, openingDate: '2020-01-01', contact: null })
  acc = await account('BCA ER')
  acc2 = await account('Jago ER')
  catExpense = await ipc.createCategory('expense', { name: 'Belanja ER', parentId: null, icon: null, color: null })
})

const input = (over: Partial<EmailRuleInputDto> = {}): EmailRuleInputDto => ({
  name: 'Toko ER', senderPattern: 'toko-er.example', subjectRegex: null, amountRegex: String.raw`Total Rp ([\d.]+)`,
  payeeRegex: null, dateRegex: null, dateFormat: null, kind: 'expense', accountId: acc, counterAccountId: null,
  defaultCategoryId: catExpense, priority: 10, enabled: true, ...over,
})

async function openSheet(props: { rule?: EmailRuleDto | null; seed?: { input: EmailRuleInputDto; sampleMessageId: string | null } | null }) {
  const w = mount(EmailRuleSheet, { props: { open: false, rule: null, seed: null }, global: { plugins: [i18n] }, attachTo: document.body })
  await w.setProps({ open: true, rule: props.rule ?? null, seed: props.seed ?? null })
  await settle()
  return w
}

describe('EmailRuleSheet', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
    toastMock.mockClear()
    toastMock.success.mockClear()
    toastMock.error.mockClear()
  })
  afterEach(() => {
    vi.restoreAllMocks()
    document.body.innerHTML = ''
  })

  it('prefills_from_rule', async () => {
    await openSheet({ rule: { ...input(), id: 'r-1', pending: 0 } })
    expect(document.body.textContent).toContain(id.emailRules.edit)
    expect($('#er-name').value).toBe('Toko ER')
    expect($('#er-sender').value).toBe('toko-er.example')
    expect($<HTMLElement>('[data-testid="er-account"]').textContent).toContain('BCA ER')
    expect($<HTMLElement>('[data-testid="er-category"]').textContent).toContain('Belanja ER')
    expect($<HTMLElement>('[data-testid="er-kind"]').textContent).toContain(id.emailRules.kind.expense)
  })

  it('patterns_collapsed_when_editing', async () => {
    await openSheet({ rule: { ...input(), id: 'r-1', pending: 0 } })
    expect($('#er-amount')).toBeNull()
    button(id.emailRules.patterns)!.click()
    await settle()
    expect($('#er-amount').value).toBe(String.raw`Total Rp ([\d.]+)`)
    document.body.innerHTML = ''
    await openSheet({ seed: { input: input(), sampleMessageId: null } })
    expect($('#er-amount')).not.toBeNull()
  })

  it('kind_switch_toggles_category_and_counter', async () => {
    await openSheet({ seed: { input: input(), sampleMessageId: null } })
    expect($('[data-testid="er-counter"]')).toBeNull()
    await pick('er-kind', id.emailRules.kind.transfer_out)
    expect($('[data-testid="er-category"]')).toBeNull()
    expect($('[data-testid="er-counter"]')).not.toBeNull()
    await pickOption($<HTMLElement>('[data-testid="er-counter"]'), 'Jago ER')
    await pick('er-kind', id.emailRules.kind.income)
    expect($('[data-testid="er-counter"]')).toBeNull()
    // Kategori pengeluaran tidak cocok untuk pemasukan: dikosongkan.
    expect($<HTMLElement>('[data-testid="er-category"]').textContent).toContain(id.emailRules.noCategory)
    const spy = vi.spyOn(ipc, 'createEmailRule')
    $<HTMLFormElement>('[data-testid="er-form"]').dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await settle()
    expect(spy.mock.calls[0]![0]).toMatchObject({ kind: 'income', counterAccountId: null, defaultCategoryId: null })
    expect(acc2).not.toBe('')
  })

  it('invalid_regex_shows_field_error', async () => {
    await openSheet({ seed: { input: input({ name: 'Rusak ER' }), sampleMessageId: null } })
    await type('#er-amount', '(')
    const w = document.body
    $<HTMLFormElement>('[data-testid="er-form"]').dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await settle()
    const err = $<HTMLElement>('[data-testid="er-error-amount"]')
    expect(err.textContent).toBe(id.emailRules.fieldError.amount)
    expect($('#er-amount').getAttribute('aria-invalid')).toBe('true')
    expect(w.querySelector('[data-testid="er-form"]')).not.toBeNull()
    expect(toastMock.error).not.toHaveBeenCalled()
    expect((await ipc.listEmailRules()).some((r) => r.name === 'Rusak ER')).toBe(false)
  })

  it('test_panel_shows_failing_step', async () => {
    await openSheet({ seed: { input: input(), sampleMessageId: null } })
    await type('#er-sample-sender', 'notif@lain.example')
    await type('#er-sample-text', 'Total Rp 5.000')
    button(id.emailRules.test)!.click()
    await settle()
    const sender = $<HTMLElement>('[data-step="sender"]')
    expect(sender.dataset.ok).toBe('false')
    expect($('[data-step="amount"]')).toBeNull()
    expect($<HTMLElement>('[data-testid="rule-test-fail"]').textContent).toContain(id.emailRules.fail.sender)
  })

  it('test_panel_shows_result', async () => {
    await openSheet({ seed: { input: input(), sampleMessageId: null } })
    await type('#er-sample-sender', 'notif@toko-er.example')
    await type('#er-sample-text', 'Halo\nTotal Rp 41.700\n')
    button(id.emailRules.test)!.click()
    await settle()
    expect($<HTMLElement>('[data-step="sender"]').dataset.ok).toBe('true')
    expect($<HTMLElement>('[data-step="amount"]').textContent).toContain('41.700')
    const items = $<HTMLElement>('[data-testid="rule-test-items"]')
    expect(items.textContent).toContain('41.700')
    expect($('[data-testid="rule-test-fail"]')).toBeNull()
    expect($<HTMLElement>('[data-testid="rule-test-matches"]').textContent).toContain('0')
  })

  it('save_reparses_and_toasts', async () => {
    __seedEmailInbox({ unparsed: [{ messageId: '<er-1@x>', sender: 'notif@reparse-er.example', subject: 'Bayar',
      receivedAt: '2026-05-09T03:00:00Z', snippet: 'Total Rp 12.000' }] })
    const inbox = useInboxStore()
    const refresh = vi.spyOn(inbox, 'refresh')
    const w = await openSheet({ seed: { input: input({ name: 'Reparse ER', senderPattern: 'reparse-er.example' }), sampleMessageId: '<er-1@x>' } })
    $<HTMLFormElement>('[data-testid="er-form"]').dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await settle()
    expect(toastMock.success).toHaveBeenCalledWith(i18n.global.t('emailRules.savedReparsed', { n: 1 }))
    expect(refresh).toHaveBeenCalled()
    expect(w.emitted('close')).toHaveLength(1)
    expect((await ipc.listUnparsedEmails()).some((u) => u.messageId === '<er-1@x>')).toBe(false)
  })
})
