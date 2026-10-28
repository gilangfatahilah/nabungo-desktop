import { describe, expect, it } from 'vitest'
import cases from '../../../crates/mail/tests/fixtures/rule-cases.json'
import { DEFAULT_RULES, apply, normalizeAmount, parseDate, suggest, trace, validate, type Mail, type RulePatterns } from './browser-rule'

const WIB = 7 * 60
const SENT_AT = 1_793_000_000

const rule = (over: Partial<RulePatterns> = {}): RulePatterns => ({
  name: 'Kasus', senderPattern: 'example.com', subjectRegex: null, amountRegex: String.raw`^Total Rp ([\d.,]+)$`,
  payeeRegex: null, dateRegex: null, dateFormat: null, ...over,
})
const mail = (from: string, subject: string, text: string, sentAt: number | null = SENT_AT): Mail => ({ from, subject, sentAt, text })

describe('rule-cases.json (paritas crates/mail/tests/rule_cases.rs)', () => {
  for (const c of cases.cases) {
    it(c.name, () => {
      const r = rule({
        senderPattern: c.rule.sender, subjectRegex: c.rule.subject, amountRegex: c.rule.amount,
        payeeRegex: c.rule.payee, dateRegex: c.rule.date, dateFormat: c.rule.dateFormat,
      })
      const t = trace(r, mail(c.email.from, c.email.subject, c.email.text, c.email.sentAt), WIB)
      expect(t).toEqual({ ...c.expect, failText: c.expect.failText ?? null })
    })
  }
})

describe('normalizeAmount (paritas normalize_amount_handles_id_and_en_formats)', () => {
  it.each([
    ['IDR 41,700.00', 41_700], ['IDR 3,000,000.00', 3_000_000], ['Rp 21.000', 21_000], ['Rp Rp500', 500],
    ['Rp 1.250.000,00', 1_250_000], ['Rp 25,00', 25],
  ])('%s → %d', (raw, want) => expect(normalizeAmount(raw)).toEqual({ ok: want }))

  it.each([
    ['Rp 25.000,50', 'amount_fractional'], ['IDR 25,000.50', 'amount_fractional'], ['1,500', 'amount_invalid'],
    ['IDR 25,000', 'amount_invalid'], ['IDR 1,500,000', 'amount_invalid'], ['1,000', 'amount_invalid'],
    ['20.5', 'amount_fractional'], ['Rp', 'amount_empty'],
  ])('%s ditolak (%s)', (raw, code) => expect(normalizeAmount(raw)).toEqual({ err: code }))
})

describe('parseDate', () => {
  it('format & nama bulan ID/EN', () => {
    expect(parseDate('09 May 2026 12:00:00', 'dd mmm yyyy')).toBe('2026-05-09')
    expect(parseDate('1 Agustus 2026', 'dd mmm yyyy')).toBe('2026-08-01')
    expect(parseDate('27/10/2026', 'dd/mm/yyyy')).toBe('2026-10-27')
    expect(parseDate('27-10-2026 08:00', 'dd-mm-yyyy')).toBe('2026-10-27')
    expect(parseDate('31/02/2026', 'dd/mm/yyyy')).toBeNull()
    expect(parseDate('27/10/26', 'dd/mm/yyyy')).toBeNull()
    expect(parseDate('27/10/2026', 'yyyy')).toBeNull()
  })
})

describe('validate', () => {
  it('rule bawaan valid', () => {
    for (const r of DEFAULT_RULES) expect(validate(r), r.name).toBeNull()
  })
  it('field pertama yang salah', () => {
    expect(validate(rule({ name: ' ' }))).toBe('name')
    expect(validate(rule({ senderPattern: '' }))).toBe('sender')
    expect(validate(rule({ subjectRegex: '(' }))).toBe('subject')
    expect(validate(rule({ amountRegex: 'Rp [0-9]+' }))).toBe('amount')
    expect(validate(rule({ payeeRegex: 'Ke .+' }))).toBe('payee')
    expect(validate(rule({ dateRegex: '(\\d+)' }))).toBe('date_format')
    expect(validate(rule({ dateFormat: 'dd/mm/yyyy' }))).toBe('date')
    expect(validate(rule({ dateRegex: '(\\d+)', dateFormat: 'yyyy' }))).toBe('date_format')
    // Sintaks yang ditolak crate regex Rust ditolak juga di browser.
    expect(validate(rule({ amountRegex: 'Rp (?=x)([\\d.]+)' }))).toBe('amount')
  })
})

describe('apply', () => {
  it('rule pertama yang cocok; gagal nominal → unparsed (null)', () => {
    const rules = [rule({ senderPattern: 'lain.example' }), rule()]
    expect(apply(rules, mail('a@example.com', 'x', 'Total Rp 5.000'), WIB)).toEqual({
      rule: 1, items: [{ amount: 5_000, date: '2026-10-26', payee: null }],
    })
    expect(apply(rules, mail('a@example.com', 'x', 'Total Rp 25,000'), WIB)).toBeNull()
    expect(apply(rules, mail('a@example.com', 'x', 'Halo'), WIB)).toBeNull()
  })
})

describe('suggest (paritas crates/mail/tests/suggest.rs)', () => {
  it('anker label dari teks', () => {
    const m = mail('Notif@bca.co.id', 'Internet Transaction Journal',
      'Transaction Date : 09 May 2026 12:00:00\nPayment to : IDM\nTotal Payment : IDR 41,700.00\n', null)
    const r = suggest(m)
    expect(r).toMatchObject({
      name: 'Internet Transaction Journal', senderPattern: 'bca.co.id', subjectRegex: '^Internet Transaction Journal$',
      amountRegex: String.raw`^Total Payment : IDR ?([\d.,]+)$`, dateRegex: String.raw`^Transaction Date : (\d{1,2} \w+ \d{4})`,
      dateFormat: 'dd mmm yyyy', kind: 'expense', priority: 10, enabled: true,
    })
    expect(trace(r, m, WIB)).toMatchObject({ items: [{ amount: 41_700, date: '2026-05-09' }] })
  })
  it('fallback nominal generik', () => {
    const m = mail('notif@toko.example', 'Bayar', 'Anda membayar Rp 15.000 di toko 7')
    const r = suggest(m)
    expect(validate(r)).toBeNull()
    expect(r.senderPattern).toBe('toko.example')
    expect(trace(r, m, WIB)).toMatchObject({ items: [{ amount: 15_000 }] })
  })
  it('subjek di-escape', () => {
    const m = mail('a@b.example', 'Transaksi (QRIS) sukses?', 'Total : Rp 5.000')
    const r = suggest(m)
    expect(validate(r)).toBeNull()
    expect(trace(r, m, WIB)).toMatchObject({ subjectOk: true, items: [{ amount: 5_000 }] })
  })
  it('nama dipotong 60 karakter; tanpa subjek → domain', () => {
    const r = suggest(mail('a@b.example', 'x'.repeat(100), 'Rp 5.000'))
    expect([...r.name].length).toBeLessThanOrEqual(60)
    expect(suggest(mail('a@b.example', '', 'Rp 5.000'))).toMatchObject({ name: 'b.example', subjectRegex: null })
  })
})
