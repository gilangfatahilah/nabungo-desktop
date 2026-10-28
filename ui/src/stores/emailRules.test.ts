import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import * as ipc from '@/ipc'
import { __seedEmailInbox } from '@/ipc/browser-ledger'
import { useEmailRulesStore } from './emailRules'
import { useInboxStore } from './inbox'

const unparsed = (messageId: string, sender: string, snippet: string) =>
  ({ messageId, sender, subject: 'Struk', receivedAt: '2026-10-26T03:00:00Z', snippet })

describe('emailRules store (fallback browser)', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('save_reloads_inbox_when_reparsed', async () => {
    __seedEmailInbox({ unparsed: [unparsed('<es1@warung>', 'notif@warung.example', 'Total : Rp 12.000')] })
    const inbox = useInboxStore()
    await inbox.reload()
    expect(inbox.unparsed.map((u) => u.messageId)).toContain('<es1@warung>')
    const s = useEmailRulesStore()
    const input = await ipc.suggestEmailRule('<es1@warung>')
    const saved = await s.save(null, { ...input, name: 'Warung ST' })
    expect(saved.reparsed).toEqual({ emails: 1, items: 1 })
    expect(s.loaded).toBe(true)
    expect(s.rules.find((r) => r.id === saved.id)).toMatchObject({ name: 'Warung ST', pending: 1 })
    expect(inbox.unparsed.map((u) => u.messageId)).not.toContain('<es1@warung>')
    expect(inbox.pending.some((c) => c.ruleName === 'Warung ST')).toBe(true)
  })

  it('update_reloads_pending_drafts', async () => {
    // Draft dihitung dari rule saat dibaca: akun/kategori baru harus tampil tanpa email yang dipindah.
    __seedEmailInbox({ unparsed: [unparsed('<es4@depot>', 'notif@depot.example', 'Total : Rp 7.000')] })
    const s = useEmailRulesStore()
    const input = { ...(await ipc.suggestEmailRule('<es4@depot>')), name: 'Depot ST', accountId: null }
    const { id } = await s.save(null, input)
    const inbox = useInboxStore()
    await inbox.reload()
    expect(inbox.pending.find((c) => c.ruleName === 'Depot ST')?.accountId).toBeNull()
    const acc = await ipc.createAccount('bank', { name: 'Depot ST', groupId: null, includeInNetWorth: true, creditLimit: null,
      statementDay: null, icon: null, color: null, openingBalance: 0, openingDate: '2020-01-01', contact: null })
    expect((await s.save(id, { ...input, accountId: acc })).reparsed.emails).toBe(0)
    expect(inbox.pending.find((c) => c.ruleName === 'Depot ST')?.accountId).toBe(acc)
  })

  it('remove_then_restore', async () => {
    __seedEmailInbox({ unparsed: [unparsed('<es2@kios>', 'notif@kios.example', 'Total : Rp 3.000')] })
    const s = useEmailRulesStore()
    const input = await ipc.suggestEmailRule('<es2@kios>')
    const { id } = await s.save(null, { ...input, name: 'Kios ST', enabled: false })
    await s.save(id, { ...input, name: 'Kios ST 2', enabled: false })
    expect(s.rules.find((r) => r.id === id)?.name).toBe('Kios ST 2')
    await s.remove(id)
    expect(s.rules.some((r) => r.id === id)).toBe(false)
    const inbox = useInboxStore()
    const restored = await s.restore(id)
    expect(restored.reparsed).toEqual({ emails: 0, items: 0 })
    expect(s.rules.find((r) => r.id === id)).toMatchObject({ enabled: false })
    expect((await s.setEnabled(id, true)).reparsed).toEqual({ emails: 1, items: 1 })
    expect(s.rules.find((r) => r.id === id)).toMatchObject({ enabled: true, pending: 1 })
    expect(inbox.summary.unparsed).toBe((await ipc.listUnparsedEmails()).length)
  })
})

describe('inbox store reparse', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('reparse_reloads_lists', async () => {
    __seedEmailInbox({ unparsed: [unparsed('<es3@warung>', 'notif@warung.example', 'Total : Rp 4.000')] })
    const inbox = useInboxStore()
    await inbox.reload()
    expect(await inbox.reparse()).toEqual({ emails: 1, items: 1 })
    expect(inbox.unparsed.map((u) => u.messageId)).not.toContain('<es3@warung>')
  })
})
