import { describe, expect, it } from 'vitest'
import type { EmailSyncDone } from './bindings'
import { createBrowserEmail } from './browser-email'

const ADDR = 'me@gmail.com'

async function connected() {
  const email = createBrowserEmail()
  expect(await email.connectEmail(ADDR, 'Finance', 'pw')).toEqual({ status: 'ok', data: null })
  return email
}

const code = (r: { status: string; error?: { code: string } }) => (r.status === 'error' ? r.error?.code : 'ok')

describe('browser-email (port email_settings.rs / email_runner.rs)', () => {
  it('belum terhubung: status default dengan label Finance', async () => {
    const r = await createBrowserEmail().emailStatus()
    expect(r).toEqual({
      status: 'ok',
      data: { enabled: false, address: null, mailbox: 'Finance', lastSyncAt: null, running: false, lastError: null, paused: false },
    })
  })

  it('alamat dinormalisasi seperti normalize_address', async () => {
    const email = createBrowserEmail()
    for (const bad of ['', 'me', 'a@@b', '@b', 'a@', 'a b@c', `${'a'.repeat(251)}@b.c`]) {
      expect(code(await email.connectEmail(bad, 'Finance', 'pw')), bad).toBe('email_address_invalid')
    }
    await email.connectEmail(' Me@Gmail.COM ', '  Bank ', 'pw')
    const s = await email.emailStatus()
    expect(s.status === 'ok' && [s.data.address, s.data.mailbox, s.data.enabled]).toEqual([ADDR, 'Bank', true])
  })

  it('label kosong ditolak; label bersarang diterima', async () => {
    const email = createBrowserEmail()
    expect(code(await email.connectEmail(ADDR, '   ', 'pw'))).toBe('email_mailbox_empty')
    expect(code(await email.connectEmail(ADDR, 'Finance/Bank', 'pw'))).toBe('ok')
  })

  it('password wrong → email_auth_failed, label Missing → email_mailbox_not_found, tanpa menyimpan', async () => {
    const email = createBrowserEmail()
    expect(code(await email.connectEmail(ADDR, 'Finance', 'wrong'))).toBe('email_auth_failed')
    expect(code(await email.connectEmail(ADDR, 'Missing', 'pw'))).toBe('email_mailbox_not_found')
    const s = await email.emailStatus()
    expect(s.status === 'ok' && [s.data.enabled, s.data.address]).toEqual([false, null])
  })

  it('tanpa password memakai yang tersimpan; belum ada → email_not_configured', async () => {
    const email = createBrowserEmail()
    expect(code(await email.connectEmail(ADDR, 'Finance', null))).toBe('email_not_configured')
    expect(code(await email.connectEmail(ADDR, 'Finance', '  '))).toBe('email_not_configured')
    await email.connectEmail(ADDR, 'Finance', 'pw')
    expect(code(await email.connectEmail(ADDR, 'Bank', null))).toBe('ok')
  })

  it('syncEmail belum terhubung → email_not_configured', async () => {
    expect(code(await createBrowserEmail().syncEmail())).toBe('email_not_configured')
  })

  it('syncEmail sukses → ringkasan nol, done terpancar, lastSyncAt terisi', async () => {
    const email = await connected()
    const done: EmailSyncDone[] = []
    const off = email.subscribe({ done: (d) => done.push(d) })
    const zero = { newItems: 0, unparsed: 0, seen: 0 }
    expect(await email.syncEmail()).toEqual({ status: 'ok', data: zero })
    expect(done).toEqual([{ summary: zero, error: null }])
    const s = await email.emailStatus()
    expect(s.status === 'ok' && s.data.lastSyncAt).toBeTruthy()
    off()
    await email.syncEmail()
    expect(done).toHaveLength(1)
  })

  it('disconnectEmail mempertahankan alamat & label, lalu sync ditolak', async () => {
    const email = await connected()
    await email.disconnectEmail()
    const s = await email.emailStatus()
    expect(s.status === 'ok' && [s.data.enabled, s.data.address, s.data.mailbox]).toEqual([false, ADDR, 'Finance'])
    expect(code(await email.syncEmail())).toBe('email_not_configured')
    // Password ikut dihapus seperti entry keychain.
    expect(code(await email.connectEmail(ADDR, 'Finance', null))).toBe('email_not_configured')
  })
})
