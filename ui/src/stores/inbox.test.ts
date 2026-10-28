import { createPinia, setActivePinia } from 'pinia'
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import * as ipc from '@/ipc'
import { __seedEmailInbox } from '@/ipc/browser-ledger'
import { useInboxStore } from './inbox'

let acc = ''
let cat = ''

const seed = (id: string) => ({
  id, kind: 'expense' as const, date: '2026-05-09', amount: 25_000, payee: 'Kopi', accountId: acc,
  toAccountId: null, categoryId: cat, ruleName: 'BCA QRIS', snippet: null,
})

beforeAll(async () => {
  acc = await ipc.createAccount('bank', { name: 'Inbox ST', groupId: null, includeInNetWorth: true, creditLimit: null,
    statementDay: null, icon: null, color: null, openingBalance: 0, openingDate: '2026-01-01', contact: null })
  cat = await ipc.createCategory('expense', { name: 'Makan ST', parentId: null, icon: null, color: null })
})

describe('inbox store (fallback browser)', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('reload_fills_lists_and_summary', async () => {
    __seedEmailInbox({
      captures: [seed('st-1')],
      unparsed: [{ messageId: '<st@x>', sender: 'bank@x', subject: 'Promo', receivedAt: null, snippet: '' }],
    })
    const s = useInboxStore()
    await s.reload()
    expect(s.loaded).toBe(true)
    expect(s.pending.map((c) => c.id)).toContain('st-1')
    expect(s.unparsed.map((u) => u.messageId)).toContain('<st@x>')
    expect(s.summary.pending).toBe(s.pending.length)
    expect(s.summary.unparsed).toBe(s.unparsed.length)
  })

  it('showNav_rules', () => {
    const s = useInboxStore()
    s.summary = { pending: 0, unparsed: 0, emailEnabled: false }
    expect(s.showNav).toBe(false)
    s.summary = { pending: 1, unparsed: 0, emailEnabled: false }
    expect(s.showNav).toBe(true)
    s.summary = { pending: 0, unparsed: 2, emailEnabled: false }
    expect(s.showNav).toBe(true)
    s.summary = { pending: 0, unparsed: 0, emailEnabled: true }
    expect(s.showNav).toBe(true)
  })

  it('quickConfirm mencatat transaksi dan mengeluarkan capture dari daftar', async () => {
    __seedEmailInbox({ captures: [seed('st-q')] })
    const s = useInboxStore()
    await s.reload()
    const id = await s.quickConfirm(s.pending.find((c) => c.id === 'st-q')!)
    expect(id).toBeTruthy()
    expect(s.pending.map((c) => c.id)).not.toContain('st-q')
  })

  it('reload_failure_keeps_confirm_result', async () => {
    __seedEmailInbox({ captures: [seed('st-r')] })
    const s = useInboxStore()
    await s.reload()
    const c = s.pending.find((x) => x.id === 'st-r')!
    const spy = vi.spyOn(ipc, 'listPendingCaptures').mockRejectedValueOnce(new Error('reload gagal'))
    try {
      await expect(s.quickConfirm(c)).resolves.toBeTruthy()
    } finally {
      spy.mockRestore()
    }
  })

  it('confirm_not_pending_reloads', async () => {
    __seedEmailInbox({ captures: [seed('st-2')] })
    const s = useInboxStore()
    await s.reload()
    const c = s.pending.find((x) => x.id === 'st-2')!
    await ipc.ignoreCapture('st-2')
    await expect(s.quickConfirm(c)).rejects.toMatchObject({ error: { code: 'capture_not_pending' } })
    expect(s.pending.map((x) => x.id)).not.toContain('st-2')
  })

  it('ignore lalu unignore mengembalikan capture', async () => {
    __seedEmailInbox({ captures: [seed('st-3')] })
    const s = useInboxStore()
    await s.reload()
    await s.ignore('st-3')
    expect(s.pending.map((x) => x.id)).not.toContain('st-3')
    await s.unignore(['st-3'])
    expect(s.pending.map((x) => x.id)).toContain('st-3')
  })

  it('listen_reloads_on_sync_done', async () => {
    await ipc.connectEmail('saya@gmail.com', 'Finance', 'abcd efgh')
    const s = useInboxStore()
    await s.listen()
    await s.listen()
    const spy = vi.spyOn(ipc, 'inboxSummary')
    await ipc.syncEmail()
    await vi.waitFor(() => expect(spy).toHaveBeenCalledTimes(1))
    await vi.waitFor(() => expect(s.summary.emailEnabled).toBe(true))
    spy.mockRestore()
    await ipc.disconnectEmail()
  })
})
