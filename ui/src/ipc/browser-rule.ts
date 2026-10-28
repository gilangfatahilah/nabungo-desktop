// Port `crates/mail/src/rule.rs`, `suggest.rs`, dan `defaults.rs` untuk mode browser (K12).
// Aturan harus tetap sama dengan Rust — ubah bersamaan; kasus paritas di
// `crates/mail/tests/fixtures/rule-cases.json` dijalankan kedua sisi.
// Regex JS dipakai tanpa flag `u` (escape Rust seperti `\#`/`\-` hanya sah tanpa `u`).
import type { EmailRuleInputDto, ErrorCode, RuleTestDto, RuleTestItemDto } from './bindings'
import { parseAmountBrowser } from './browser-amount'

export type RulePatterns = Pick<
  EmailRuleInputDto,
  'name' | 'senderPattern' | 'subjectRegex' | 'amountRegex' | 'payeeRegex' | 'dateRegex' | 'dateFormat'
>
/** `MailMessage`: `sentAt` = detik unix header `Date` (null bila tidak ada). */
export type Mail = { from: string; subject: string; sentAt: number | null; text: string }
export type Trace = Omit<RuleTestDto, 'unparsedMatches'>
/** Sama dengan `entity` error `email_rule_invalid`. */
export type RuleField = 'name' | 'sender' | 'subject' | 'amount' | 'payee' | 'date' | 'date_format'

export const DATE_FORMATS = ['dd/mm/yyyy', 'dd-mm-yyyy', 'dd mmm yyyy'] as const

/** Sintaks yang ditolak crate `regex` Rust (lookaround, backreference) tetapi diterima JS. */
const RUST_UNSUPPORTED = /\(\?<?[=!]|\\[1-9]/

/** `regex()`: case-insensitive + multi-line; `null` bila tidak valid. */
function regex(pattern: string, global = false): RegExp | null {
  if (RUST_UNSUPPORTED.test(pattern)) return null
  try {
    return new RegExp(pattern, global ? 'gim' : 'im')
  } catch {
    return null
  }
}

const groups = (re: RegExp) => new RegExp(`${re.source}|`).exec('')!.length - 1

export function validate(r: RulePatterns): RuleField | null {
  const withGroup = (p: string) => {
    const re = regex(p)
    return re !== null && groups(re) >= 1
  }
  if (r.name.trim() === '') return 'name'
  if (r.senderPattern.trim() === '') return 'sender'
  if (r.subjectRegex !== null && regex(r.subjectRegex) === null) return 'subject'
  if (!withGroup(r.amountRegex)) return 'amount'
  if (r.payeeRegex !== null && !withGroup(r.payeeRegex)) return 'payee'
  if (r.dateRegex !== null && r.dateFormat !== null) {
    if (!withGroup(r.dateRegex)) return 'date'
    if (!(DATE_FORMATS as readonly string[]).includes(r.dateFormat)) return 'date_format'
  } else if (r.dateRegex !== null) return 'date_format'
  else if (r.dateFormat !== null) return 'date'
  return null
}

type Compiled = {
  sender: string
  subject: RegExp | null
  amount: RegExp
  payee: RegExp | null
  date: [RegExp, string] | null
}

function compile(r: RulePatterns): Compiled | null {
  const amount = regex(r.amountRegex, true)
  const subject = r.subjectRegex === null ? null : regex(r.subjectRegex)
  const payee = r.payeeRegex === null ? null : regex(r.payeeRegex)
  const date = r.dateRegex !== null && r.dateFormat !== null ? regex(r.dateRegex) : null
  if (amount === null || (r.subjectRegex !== null && subject === null) || (r.payeeRegex !== null && payee === null)) return null
  if (r.dateRegex !== null && r.dateFormat !== null && date === null) return null
  return {
    sender: r.senderPattern.trim().toLowerCase(),
    subject, amount, payee,
    date: date === null ? null : [date, r.dateFormat!],
  }
}

/** `trace`: rule tidak valid → nama field-nya. */
export function trace(r: RulePatterns, msg: Mail, offsetMinutes: number): Trace | RuleField {
  const invalid = validate(r)
  if (invalid !== null) return invalid
  const c = compile(r)
  return c === null ? 'amount' : run(c, msg.from.toLowerCase(), msg, offsetMinutes)
}

function run(rule: Compiled, from: string, msg: Mail, offsetMinutes: number): Trace {
  const senderOk = from.includes(rule.sender)
  const subjectOk = rule.subject === null ? null : rule.subject.test(msg.subject)
  const t: Trace = {
    senderOk, subjectOk, amounts: [], payee: null, dateText: null, dateFromHeader: false,
    fail: 'sender', failText: null, items: [],
  }
  if (!senderOk) return t
  if (subjectOk === false) return { ...t, fail: 'subject' }
  t.amounts = [...msg.text.matchAll(rule.amount)].map((m) => m[1]).filter((g): g is string => g !== undefined)
  if (t.amounts.length === 0) return { ...t, fail: 'no_amount' }
  const p = rule.payee?.exec(msg.text)?.[1]?.trim()
  t.payee = p ? p : null
  let bodyDate: string | null = null
  if (rule.date !== null) {
    const [re, fmt] = rule.date
    t.dateText = re.exec(msg.text)?.[1] ?? null
    bodyDate = t.dateText === null ? null : parseDate(t.dateText, fmt)
  }
  const headerDate = msg.sentAt === null ? null : new Date((msg.sentAt + offsetMinutes * 60) * 1000).toISOString().slice(0, 10)
  let date: string
  if (bodyDate !== null) date = bodyDate
  else if (headerDate !== null) {
    t.dateFromHeader = true
    date = headerDate
  } else {
    return t.dateText === null ? { ...t, fail: 'no_date' } : { ...t, fail: 'date_invalid', failText: t.dateText }
  }
  const items: RuleTestItemDto[] = []
  for (const raw of t.amounts) {
    const a = normalizeAmount(raw)
    if (!('ok' in a) || a.ok <= 0) return { ...t, fail: 'amount_invalid', failText: raw }
    items.push({ amount: a.ok, date, payee: t.payee })
  }
  return { ...t, fail: null, items }
}

/** `CompiledRules::apply`: rule pertama yang tidak dilewati; `null` = unparsed. */
export function apply(rules: RulePatterns[], msg: Mail, offsetMinutes: number): { rule: number; items: RuleTestItemDto[] } | null {
  const from = msg.from.toLowerCase()
  for (const [i, r] of rules.entries()) {
    const c = compile(r)
    if (c === null || c.sender === '') continue
    const t = run(c, from, msg, offsetMinutes)
    if (t.fail === null) return { rule: i, items: t.items }
    if (t.fail === 'sender' || t.fail === 'subject' || t.fail === 'no_amount') continue
    return null
  }
  return null
}

/** `normalize_amount`: format Indonesia maupun Inggris; koma tunggal 3 digit ambigu → ditolak. */
export function normalizeAmount(raw: string): { ok: number } | { err: ErrorCode } {
  const s = raw.replace(/[^0-9.,]/g, '').replace(/^[.,]+|[.,]+$/g, '')
  const dot = s.lastIndexOf('.')
  const comma = s.lastIndexOf(',')
  const count = (ch: string) => s.split(ch).length - 1
  if (dot < 0 && comma >= 0 && (count(',') > 1 || s.length - comma - 1 > 2)) return { err: 'amount_invalid' }
  let decimal = -1
  if (dot >= 0 && comma >= 0) decimal = Math.max(dot, comma)
  else if (comma >= 0) decimal = comma
  else if (dot >= 0 && count('.') === 1 && s.length - dot - 1 !== 3) decimal = dot
  const id = [...s].map((c, i) => (c === '.' || c === ',' ? (i === decimal ? ',' : '.') : c)).join('')
  const r = parseAmountBrowser(id)
  return 'ok' in r ? { ok: r.ok } : { err: r.err }
}

const MONTHS: Record<string, number> = {
  jan: 1, feb: 2, peb: 2, mar: 3, apr: 4, mei: 5, may: 5, jun: 6, jul: 7,
  agu: 8, aug: 8, sep: 9, okt: 10, oct: 10, nov: 11, des: 12, dec: 12,
}

function monthFromName(name: string): number | null {
  const n = name.toLowerCase()
  if ([...n].length < 3 || !/^\p{L}+$/u.test(n)) return null
  return MONTHS[n.slice(0, 3)] ?? null
}

const smallInt = (s: string | undefined) => (s !== undefined && /^\+?\d+$/.test(s) && Number(s) <= 255 ? Number(s) : null)

/** `parse_date` → `YYYY-MM-DD`; teks setelah tanggal diabaikan. */
export function parseDate(text: string, format: string): string | null {
  const t = text.trim()
  let day: number | null, month: number | null, rest: string | undefined
  if (format === 'dd/mm/yyyy' || format === 'dd-mm-yyyy') {
    const sep = format[2]!
    const i = t.indexOf(sep)
    const j = i < 0 ? -1 : t.indexOf(sep, i + 1)
    if (j < 0) return null
    day = smallInt(t.slice(0, i))
    const m = smallInt(t.slice(i + 1, j))
    month = m !== null && m >= 1 && m <= 12 ? m : null
    rest = t.slice(j + 1)
  } else if (format === 'dd mmm yyyy') {
    const parts = t.split(/[ ,]/).filter((p) => p !== '')
    day = smallInt(parts[0])
    month = parts[1] === undefined ? null : monthFromName(parts[1])
    rest = parts[2]
  } else return null
  if (day === null || month === null || rest === undefined) return null
  const year = /^\d*/.exec(rest)![0]
  if (year.length !== 4) return null
  const d = new Date(Date.UTC(Number(year), month - 1, day))
  if (d.getUTCMonth() !== month - 1 || d.getUTCDate() !== day) return null
  return `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`
}

// --- suggest.rs ---

const NAME_CHARS = 60
const LABEL_CHARS = 40
const PRIORITY = 10
const GENERIC_AMOUNT = String.raw`(?:Rp\.?|IDR) ?([\d.,]+)`
const AMOUNT = /(Rp\.?|IDR) ?(\d[\d.,]*)/gi
const DATES: [RegExp, string, string][] = [
  [/\d{1,2} [A-Za-z]+ \d{4}/, String.raw`(\d{1,2} \w+ \d{4})`, 'dd mmm yyyy'],
  [/\d{1,2}\/\d{1,2}\/\d{4}/, String.raw`(\d{1,2}/\d{1,2}/\d{4})`, 'dd/mm/yyyy'],
  [/\d{1,2}-\d{1,2}-\d{4}/, String.raw`(\d{1,2}-\d{1,2}-\d{4})`, 'dd-mm-yyyy'],
]

/** `regex::escape`. */
const escape = (s: string) => s.replace(/[\\.+*?()|[\]{}^$#&\-~]/g, '\\$&')
const lines = (text: string) => text.split('\n').map((l) => l.replace(/\r$/, ''))
const stableLabel = (prefix: string) => [...prefix].length <= LABEL_CHARS && !/[0-9]/.test(prefix)

function suggestAmount(text: string): string | null {
  for (const line of lines(text)) {
    const m = [...line.matchAll(AMOUNT)].find((c) => {
      const a = normalizeAmount(c[2]!)
      return 'ok' in a && a.ok > 0
    })
    if (!m) continue
    const prefix = line.slice(0, m.index)
    if (prefix.trim() === '' || !stableLabel(prefix)) continue
    const end = line.slice(m.index + m[0].length).trim() === '' ? '$' : ''
    return `^${escape(prefix)}${escape(m[1]!)} ?([\\d.,]+)${end}`
  }
  return null
}

function suggestDate(text: string): [string, string] | null {
  for (const line of lines(text)) {
    for (const [re, pattern, format] of DATES) {
      const m = re.exec(line)
      if (!m) continue
      const prefix = line.slice(0, m.index)
      if (prefix.trim() === '' || !stableLabel(prefix) || parseDate(m[0], format) === null) continue
      return [`^${escape(prefix)}${pattern}`, format]
    }
  }
  return null
}

export function suggest(msg: Mail): EmailRuleInputDto {
  const at = msg.from.lastIndexOf('@')
  const domain = (at < 0 ? msg.from : msg.from.slice(at + 1)).trim()
  const subject = msg.subject.trim()
  const date = suggestDate(msg.text)
  return {
    name: subject === '' ? domain : [...subject].slice(0, NAME_CHARS).join('').trim(),
    senderPattern: domain,
    subjectRegex: subject === '' ? null : `^${escape(subject)}$`,
    amountRegex: suggestAmount(msg.text) ?? GENERIC_AMOUNT,
    payeeRegex: null,
    dateRegex: date?.[0] ?? null,
    dateFormat: date?.[1] ?? null,
    kind: 'expense',
    accountId: null,
    counterAccountId: null,
    defaultCategoryId: null,
    priority: PRIORITY,
    enabled: true,
  }
}

// --- defaults.rs ---

const BCA = 'bca.co.id'
const JAGO = 'jago.com'
const BCA_DATE = String.raw`^Transaction Date : (\d{1,2} \w+ \d{4})`

const def = (
  name: string, senderPattern: string, subject: string, amountRegex: string, payeeRegex: string | null,
  dateRegex: string | null, kind: EmailRuleInputDto['kind'],
): EmailRuleInputDto => ({
  name, senderPattern, subjectRegex: subject, amountRegex, payeeRegex, dateRegex,
  dateFormat: dateRegex === null ? null : 'dd mmm yyyy', kind,
  accountId: null, counterAccountId: null, defaultCategoryId: null, priority: 0, enabled: true,
})

/** `default_rules()`: tanpa akun/kategori, priority 0. */
export const DEFAULT_RULES: readonly EmailRuleInputDto[] = [
  def('BCA Pembayaran (QRIS/VA)', BCA, '^Internet Transaction Journal$', String.raw`^Total Payment : IDR ([\d.,]+)$`,
    '^(?:Payment to|Company/Product Name) : (.+)$', BCA_DATE, 'expense'),
  def('BCA Transfer', BCA, '^Internet Transaction Journal$', String.raw`^Transfer Amount : IDR ([\d.,]+)$`,
    '^Beneficiary Name : (.+)$', BCA_DATE, 'expense'),
  def('BCA Tarik Tunai', BCA, '^Cash Withdrawal Successful$', String.raw`^Amount : IDR ([\d.,]+)$`, null, BCA_DATE, 'transfer_out'),
  def('Jago Pembayaran', JAGO, '^Kamu telah membayar ke ', String.raw`^Jumlah Rp ?([\d.,]+)$`, '^Ke (.+?),?$',
    String.raw`^Tanggal Transaksi (\d{1,2} \w+ \d{4})`, 'expense'),
  def('Jago Kartu Debit', JAGO, 'kartu debit Jago$', String.raw`sebesar Rp ?([\d.,]+) menggunakan kartu debit`, null, null, 'expense'),
  def('Jago Uang Dikembalikan', JAGO, '^Uang telah dikembalikan$', String.raw`sebesar (?:Rp ?)+([\d.,]+) ke `,
    String.raw`sebesar (?:Rp ?)+[\d.,]+ ke (.+?)\.?$`, null, 'income'),
  def('Jago Terima Uang', JAGO, '^Kamu telah menerima uang di Jago', String.raw`menerima Rp ?([\d.,]+) dari`,
    String.raw`menerima Rp ?[\d.,]+ dari akun (.+?)\.?$`, null, 'transfer_in'),
]
