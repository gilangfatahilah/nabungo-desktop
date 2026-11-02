// Ledger in-memory untuk mode browser (tanpa Tauri): cukup agar layar bisa dicoba.
// Aturan bisnis yang sebenarnya ada di Rust; aturan transaksi di sini (validasi form, saldo,
// saldo berjalan, filter) dicerminkan dari core/storage dan diuji di browser-ledger.test.ts.
import type {
  AccountDto, AccountFieldsDto, AccountGroupDto, AccountKindDto, AccountsOverviewDto, AllocationOverviewDto,
  AllocationPointDto, AmountWarningDto, BackupDto, BackupKindDto, BackupSecretDto, BackupSourceDto, BackupSourceInfoDto, BucketActualDto, CashflowPointDto, CategoryAmountDto, CategoryComparisonDto, CategoryDeltaDto, CategoryTrendPointDto, NetWorthPointDto, BucketDto, BucketFieldsDto, BudgetOverviewDto, MappingDto, ShareDto,
  BudgetAlertDto, BudgetLevelDto, SavedTxnDto,
  UnallocatedDto, WarningDto,
  BudgetRowDto, CategoryDto,
  CategoryFieldsDto, CategoryKindDto, ContactDto, ErrorCode, IpcError, OnboardingDto, PayeeSuggestionDto, PostingDto,
  TxnDetailDto, TxnFormDefaultsDto, TxnFormDto, TxnKindDto, TxnPageDto, TxnQueryDto, TxnRowDto, TxnSortDto,
  ReconciliationDto, RecoveryReasonDto, RecoveryStatusDto, SecurityStatusDto, StartupReportDto, ValuationPointDto,
  AttachmentDto, PayeeUsageDto, TagUsageDto,
  RecurrenceDto, RuleDto, RuleInputDto,
  CaptureDto, DuplicateTxnDto, InboxSummaryDto, UnparsedEmailDto,
  EmailRuleDto, EmailRuleInputDto, ReparseSummaryDto, RuleSampleDto, RuleSavedDto, RuleTestDto,
} from './bindings'
import { DEFAULT_RULES, apply, pasted, suggest, trace, validate, type Mail, type RulePatterns } from './browser-rule'
import { keepDay, nextAfter, occurrences } from './browser-recurrence'
import { todayIso } from '@/lib/date'
import { countsAsLiability, isLiability } from '@/lib/accounts'

type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: IpcError }

const ok = <T>(data: T): Result<T> => ({ status: 'ok', data })
const fail = (code: ErrorCode, entity: string | null = null): Result<never> => ({ status: 'error', error: { code, entity } })
const notFound = (entity: string): Result<never> => fail('not_found', entity)

const state = {
  onboarded: false,
  periodStartDay: 1,
  groups: [] as AccountGroupDto[],
  accounts: [] as AccountDto[],
  categories: [] as CategoryDto[],
  txns: [] as Txn[],
  payees: [] as { id: string; name: string; defaultCategoryId: string | null }[],
  /** Registri tag seperti tabel `tag` di Rust: tag tak terpakai tetap tercantum. */
  tags: [] as { id: string; name: string }[],
  /** Lampiran simulasi (tanpa filesystem). */
  attachments: [] as (AttachmentDto & { txnId: string })[],
  contacts: [] as ContactDto[],
  lastFeeCategoryId: null as string | null,
  budgets: [] as { categoryId: string; period: string; amount: number; rollover: boolean }[],
  buckets: [] as BucketDto[],
  mappings: [] as MappingDto[],
  rules: [] as Rule[],
  captures: [] as Capture[],
  /** Terbaru di depan, seperti `ORDER BY created_at DESC`. */
  unparsed: [] as UnparsedEmailDto[],
  /** Tabel `email_rule` (bukan `rules`, itu transaksi terjadwal). */
  emailRules: [] as EmailRule[],
  /** Setting `email.rules_seeded`. */
  emailRulesSeeded: false,
}

/** Isi DB baru (sebelum onboarding), untuk `recoveryStartFresh`. */
const FRESH_STATE = structuredClone(state)

/** Seperti `recurring_rule`: `anchor` = tanggal mulai (sumber BYMONTHDAY/BYDAY di rrule Rust). */
type Rule = {
  id: string
  form: TxnFormDto
  recurrence: RecurrenceDto
  autoPost: boolean
  anchor: string
  next: string
  ended: boolean
}

// Sama dengan `MAX_CATCH_UP` / `MAX_INTERVAL` di Rust.
const MAX_CATCH_UP = 366
const MAX_INTERVAL = 99

/** Baris `email_capture`; kandidat duplikat dihitung saat dibaca (K14). */
/** `ruleId` terisi untuk capture hasil re-parse: draft diturunkan dari rule saat dibaca (`draft_of`). */
type Capture = Omit<CaptureDto, 'duplicates'> & { status: 'pending' | 'confirmed' | 'ignored'; txnId: string | null; ruleId?: string }

type EmailRule = EmailRuleInputDto & { id: string; deleted: boolean }

export type BrowserCaptureSeed = Omit<CaptureDto, 'id' | 'duplicates'> & { id?: string }
export type BrowserUnparsedSeed = UnparsedEmailDto

type Txn = {
  id: string
  seq: number
  kind: TxnKindDto
  date: string
  payeeId: string | null
  note: string | null
  tags: string[]
  postings: PostingDto[]
  /** Akun yang posting-nya sudah ditandai rekonsiliasi (ACC-04); edit mengosongkannya. */
  reconciled: string[]
  deleted: boolean
}

// Batas nominal per posting, sama dengan `MAX_AMOUNT` di core.
const MAX_AMOUNT = 1_000_000_000_000
// Sama dengan `MAX_NAME_LEN` di core/catalog.rs.
const MAX_NAME_LEN = 80
const SEED_FEE_NAMES = ['biaya admin', 'fees']
let seq = 0

const id = () => crypto.randomUUID()

function nextSort(items: { sortOrder: number }[]): number {
  return items.reduce((m, x) => Math.max(m, x.sortOrder), 0) + 1
}

/** Seperti `ensure_contact`: kontak hanya untuk akun piutang/hutang, dipakai ulang tanpa beda huruf. */
function resolveContact(kind: AccountKindDto, name: string | null): string | null {
  const n = kind === 'receivable' ? clean(name) : null
  if (!n) return null
  const found = state.contacts.find((c) => c.name.toLowerCase() === n.toLowerCase())
  if (found) return found.name
  state.contacts.push({ id: id(), name: n })
  return n
}

function toAccount(kind: AccountKindDto, f: AccountFieldsDto, base: Pick<AccountDto, 'id' | 'sortOrder' | 'archived'>): AccountDto {
  const credit = kind === 'credit_card'
  return {
    ...base,
    kind,
    name: f.name.trim(),
    groupId: f.groupId,
    includeInNetWorth: f.includeInNetWorth,
    creditLimit: credit ? f.creditLimit : null,
    statementDay: credit ? f.statementDay : null,
    icon: f.icon,
    color: f.color,
    // Diisi ulang dari posting di `overview()`.
    balance: 0,
    openingBalance: f.openingBalance,
    openingDate: f.openingBalance !== 0 ? f.openingDate : null,
    contact: resolveContact(kind, f.contact),
    // Diisi ulang di `overview()`.
    reconciledThrough: null,
  }
}

const activeTxns = () => state.txns.filter((t) => !t.deleted)

/** Saldo dari postings; dengan `asOf`, hanya transaksi bertanggal ≤ `asOf` (inklusif). */
function balanceOf(accountId: string, asOf: string | null = null): number {
  let sum = 0
  for (const t of activeTxns()) {
    if (asOf !== null && t.date > asOf) continue
    for (const p of t.postings) if (p.accountId === accountId) sum += p.amount
  }
  return sum
}

/** Seperti Rust: tanggal transaksi aktif terakhir yang posting akunnya sudah direkonsiliasi. */
function reconciledThrough(accountId: string): string | null {
  return activeTxns().filter((t) => t.reconciled.includes(accountId))
    .reduce<string | null>((m, t) => (m === null || t.date > m ? t.date : m), null)
}

/** Seperti Rust: saldo awal ≠ 0 disimpan sebagai transaksi `opening`. */
// Seperti Rust: saldo awal hanya ditulis ulang bila nominal/tanggal berubah, agar tanda reconciled tetap.
function syncOpening(a: AccountDto) {
  const cur = state.txns.find((t) => t.kind === 'opening' && !t.deleted && t.postings[0]?.accountId === a.id)
  const wanted = a.openingBalance !== 0 && a.openingDate
  if (cur && wanted && cur.postings[0]!.amount === a.openingBalance && cur.date === a.openingDate) return
  if (!cur && !wanted) return
  state.txns = state.txns.filter((t) => !(t.kind === 'opening' && t.postings[0]?.accountId === a.id))
  if (a.openingBalance === 0 || !a.openingDate) return
  state.txns.push({
    id: id(), seq: ++seq, kind: 'opening', date: a.openingDate, payeeId: null, note: null, tags: [],
    postings: [{ accountId: a.id, categoryId: null, amount: a.openingBalance, memo: null }], reconciled: [], deleted: false,
  })
}

function overview(): AccountsOverviewDto {
  let assets = 0
  let liabilities = 0
  for (const a of state.accounts) {
    a.balance = balanceOf(a.id)
    a.reconciledThrough = reconciledThrough(a.id)
  }
  for (const a of state.accounts) {
    // Seperti Rust: akun diarsip tetap dihitung; hanya `includeInNetWorth` yang menyaring.
    if (!a.includeInNetWorth) continue
    if (countsAsLiability(a.kind, a.balance)) liabilities += -a.balance
    else assets += a.balance
  }
  return {
    groups: state.groups.map((g) => ({ ...g })),
    accounts: state.accounts.map((a) => ({ ...a })),
    netWorth: { assets, liabilities, net: assets - liabilities },
  }
}

function insertCategory(kind: CategoryKindDto, f: CategoryFieldsDto): string {
  const siblings = state.categories.filter((c) => c.kind === kind && c.parentId === f.parentId)
  const c: CategoryDto = {
    id: id(), kind, parentId: f.parentId, name: f.name.trim(), icon: f.icon, color: f.color,
    sortOrder: nextSort(siblings), archived: false, inUse: false,
  }
  state.categories.push(c)
  return c.id
}

function seedCategories() {
  if (state.categories.length) return
  const food = insertCategory('expense', { name: 'Makan & Minum', parentId: null, icon: 'utensils', color: null })
  insertCategory('expense', { name: 'Restoran', parentId: food, icon: 'utensils', color: null })
  insertCategory('expense', { name: 'Kopi & jajan', parentId: food, icon: 'utensils', color: null })
  insertCategory('expense', { name: 'Transportasi', parentId: null, icon: 'car', color: null })
  insertCategory('expense', { name: 'Tagihan', parentId: null, icon: 'receipt', color: null })
  insertCategory('expense', { name: 'Biaya admin', parentId: null, icon: 'landmark', color: null })
  insertCategory('income', { name: 'Gaji', parentId: null, icon: 'wallet', color: null })
  insertCategory('income', { name: 'Bonus', parentId: null, icon: 'gift', color: null })
}

function reorder<T extends { id: string; sortOrder: number }>(items: T[], ids: string[]) {
  ids.forEach((i, n) => {
    const it = items.find((x) => x.id === i)
    if (it) it.sortOrder = n + 1
  })
}

function setArchived<T extends { id: string; archived: boolean }>(items: T[], i: string, archived: boolean, entity: string): Result<null> {
  const it = items.find((x) => x.id === i)
  if (!it) return notFound(entity)
  it.archived = archived
  return ok(null)
}

// --- Transaksi (cermin `form_postings` + `validate` di core) ---

function formPostings(f: TxnFormDto): Result<PostingDto[]> {
  if (f.kind === 'transfer') {
    if (!f.toAccountId) return fail('invalid_id', 'account')
    if (f.amount <= 0) return fail('txn_not_positive')
    const ps: PostingDto[] = [
      { accountId: f.accountId, categoryId: null, amount: -f.amount, memo: null },
      { accountId: f.toAccountId, categoryId: null, amount: f.amount, memo: null },
    ]
    if (f.fee) {
      if (f.fee.amount <= 0) return fail('txn_not_positive')
      if (!f.fee.categoryId) return fail('txn_missing_category')
      ps.push({ accountId: f.accountId, categoryId: f.fee.categoryId, amount: -f.fee.amount, memo: null })
    }
    return ok(ps)
  }
  if (f.amount === 0) return fail('txn_zero_amount')
  if (!f.splits.length) return fail('txn_missing_category')
  if (f.splits.some((s) => s.amount === 0)) return fail('txn_zero_amount')
  if (f.splits.reduce((m, s) => m + s.amount, 0) !== f.amount) return fail('txn_split_mismatch')
  const sign = f.kind === 'income' ? 1 : -1
  return ok(f.splits.map((s) => ({ accountId: f.accountId, categoryId: s.categoryId, amount: s.amount * sign, memo: s.memo })))
}

/** `archivedOk`: ref milik versi lama transaksi yang boleh tetap dipakai saat diubah. */
function validatePostings(kind: TxnKindDto, ps: PostingDto[], archivedOk: Set<string>): Result<null> {
  for (const p of ps) {
    const a = state.accounts.find((x) => x.id === p.accountId)
    if (!a) return notFound('account')
    if (a.archived && !archivedOk.has(a.id)) return fail('archived', 'account')
    if (Math.abs(p.amount) > MAX_AMOUNT) return fail('txn_amount_too_large')
    if (p.categoryId === null) continue
    const c = state.categories.find((x) => x.id === p.categoryId)
    if (!c) return notFound('category')
    if (c.archived && !archivedOk.has(c.id)) return fail('archived', 'category')
    const want = kind === 'income' ? 'income' : 'expense'
    if (c.kind !== want) return fail('txn_wrong_category_kind')
  }
  if (kind === 'transfer' && ps[0]!.accountId === ps[1]!.accountId) return fail('txn_transfer_same_account')
  return ok(null)
}

const clean = (s: string | null) => (s?.trim() ? s.trim() : null)

function resolvePayee(name: string | null): string | null {
  const n = clean(name)
  if (!n) return null
  const found = state.payees.find((p) => p.name.toLowerCase() === n.toLowerCase())
  if (found) return found.id
  const p = { id: id(), name: n, defaultCategoryId: null }
  state.payees.push(p)
  return p.id
}

const sameName = (a: string, b: string) => a.toLowerCase() === b.toLowerCase()
const byName = (a: { name: string }, b: { name: string }) => a.name.toLowerCase().localeCompare(b.name.toLowerCase())

/** Cermin `normalize_name` (core). */
function catalogName(raw: string): Result<string> {
  const n = raw.trim()
  if (!n) return fail('name_empty')
  if ([...n].length > MAX_NAME_LEN) return fail('name_too_long')
  return ok(n)
}

/** Cermin `normalize_tag`: buang `#` di depan lalu trim. */
const tagName = (raw: string) => catalogName(raw.trim().replace(/^#+/, ''))

/** Tag transaksi: nama yang sudah terdaftar dipakai apa adanya (NOCASE), yang baru didaftarkan. */
function normalizeTags(tags: string[]): string[] {
  const out: string[] = []
  for (const raw of tags) {
    const r = tagName(raw)
    if (r.status === 'error') continue
    let t = state.tags.find((x) => sameName(x.name, r.data))
    if (!t) {
      t = { id: id(), name: r.data }
      state.tags.push(t)
    }
    if (!out.includes(t.name)) out.push(t.name)
  }
  return out.sort((a, b) => a.localeCompare(b))
}

/** Ganti tag `from` di semua transaksi (termasuk terhapus); `to = null` berarti dibuang. */
function retag(from: string, to: string | null) {
  for (const t of state.txns) {
    if (!t.tags.includes(from)) continue
    const rest = t.tags.filter((x) => x !== from)
    if (to && !rest.includes(to)) rest.push(to)
    t.tags = rest.sort((a, b) => a.localeCompare(b))
  }
}

function checkDefaultCategory(categoryId: string | null): Result<null> {
  if (categoryId === null) return ok(null)
  const c = state.categories.find((x) => x.id === categoryId)
  if (!c) return notFound('category')
  if (c.archived) return fail('archived', 'category')
  return ok(null)
}

function writeTxn(existing: Txn | null, f: TxnFormDto): Result<string> {
  const built = formPostings(f)
  if (built.status === 'error') return built
  const archivedOk = new Set(existing?.postings.flatMap((p) => [p.accountId, p.categoryId ?? '']) ?? [])
  const valid = validatePostings(f.kind, built.data, archivedOk)
  if (valid.status === 'error') return valid
  const fields = {
    kind: f.kind, date: f.date, payeeId: resolvePayee(f.payee), note: clean(f.note),
    tags: normalizeTags(f.tags), postings: built.data.map((p) => ({ ...p, memo: clean(p.memo) })),
  }
  if (f.kind === 'transfer' && f.fee) state.lastFeeCategoryId = f.fee.categoryId
  if (existing) {
    // K8: isi berubah, tanda rekonsiliasi tidak lagi berlaku.
    Object.assign(existing, fields, { reconciled: [] })
    return ok(existing.id)
  }
  const t: Txn = { id: id(), seq: ++seq, deleted: false, reconciled: [], ...fields }
  state.txns.push(t)
  return ok(t.id)
}

/** Cermin `rule_write`: aturan + template divalidasi seperti mencatat di tanggal mulai, tanpa menyimpan. */
function checkRule(input: RuleInputDto): Result<null> {
  const r = input.recurrence
  if (!Number.isInteger(r.interval) || r.interval < 1 || r.interval > MAX_INTERVAL) return fail('recurrence_interval')
  if (r.until !== null && r.until < input.form.date) return fail('recurrence_until')
  const built = formPostings(input.form)
  if (built.status === 'error') return built
  const valid = validatePostings(input.form.kind, built.data, new Set())
  if (valid.status === 'error') return valid
  for (const t of input.form.tags) {
    const n = tagName(t)
    if (n.status === 'error') return n
  }
  return ok(null)
}

/** Seperti `due_dates`: dari `next` selama ≤ hari ini, maksimal MAX_CATCH_UP. */
function dueDates(r: Rule, today: string): string[] {
  const out: string[] = []
  let cur: string | null = r.ended ? null : r.next
  while (cur !== null && cur <= today && out.length < MAX_CATCH_UP) {
    out.push(cur)
    cur = nextAfter(r.recurrence, r.anchor, cur)
  }
  return out
}

/** Catat satu kejadian lalu majukan jadwal; gagal → jadwal tidak berubah. */
function postOccurrence(r: Rule, date: string, form: TxnFormDto | null): Result<string> {
  const w = writeTxn(null, form ?? { ...r.form, date })
  if (w.status === 'error') return w
  const next = nextAfter(r.recurrence, r.anchor, date)
  if (next === null) r.ended = true
  else r.next = next
  return w
}

function postAllDue(r: Rule, today: string): Result<number> {
  const dates = dueDates(r, today)
  // Atomik seperti satu SQLite transaction: template sama untuk semua tanggal, cukup validasi sekali.
  if (dates.length) {
    const built = formPostings(r.form)
    if (built.status === 'error') return built
    const valid = validatePostings(r.form.kind, built.data, new Set())
    if (valid.status === 'error') return valid
  }
  for (const d of dates) {
    const w = postOccurrence(r, d, null)
    if (w.status === 'error') return w
  }
  return ok(dates.length)
}

function pendingRule(id: string, date: string): Result<Rule> {
  const r = state.rules.find((x) => x.id === id)
  if (!r) return notFound('recurring_rule')
  if (r.ended || r.next !== date) return fail('occurrence_stale')
  return ok(r)
}

const ruleDto = (r: Rule, today: string): RuleDto => ({
  id: r.id, form: { ...r.form, date: r.next, tags: [...r.form.tags] }, recurrence: { ...r.recurrence },
  autoPost: r.autoPost, ended: r.ended, due: dueDates(r, today),
})

const byNewest = (a: Txn, b: Txn) => b.date.localeCompare(a.date) || b.seq - a.seq
const attachmentDto = ({ txnId: _, ...a }: AttachmentDto & { txnId: string }): AttachmentDto => a

const payeeName = (t: Txn) => state.payees.find((p) => p.id === t.payeeId)?.name ?? null

function matches(t: Txn, q: TxnQueryDto): boolean {
  if (q.dateFrom && t.date < q.dateFrom) return false
  if (q.dateTo && t.date > q.dateTo) return false
  if (q.kind && t.kind !== q.kind) return false
  if (q.accountId && !t.postings.some((p) => p.accountId === q.accountId)) return false
  if (q.categoryId) {
    const hit = t.postings.some((p) => {
      const c = state.categories.find((x) => x.id === p.categoryId)
      return c && (c.id === q.categoryId || c.parentId === q.categoryId)
    })
    if (!hit) return false
  }
  if (q.tag && !t.tags.some((g) => g.toLowerCase() === q.tag!.toLowerCase())) return false
  if (q.payeeId && t.payeeId !== q.payeeId) return false
  const amount = Math.abs(rowAmount(t))
  if (q.amountMin !== null && amount < q.amountMin) return false
  if (q.amountMax !== null && amount > q.amountMax) return false
  const needle = q.search?.trim().toLowerCase()
  if (needle) {
    const hay = [payeeName(t), t.note, ...t.postings.map((p) => p.memo)]
    if (!hay.some((h) => h?.toLowerCase().includes(needle))) return false
  }
  return true
}

function rowAmount(t: Txn): number {
  if (t.kind === 'transfer') return Math.max(...t.postings.filter((p) => !p.categoryId).map((p) => p.amount))
  return t.postings.reduce((m, p) => m + p.amount, 0)
}

function shapeRow(t: Txn, running: Map<string, number> | null, accountId: string | null): TxnRowDto {
  const row: TxnRowDto = {
    id: t.id, kind: t.kind, date: t.date, payee: payeeName(t), note: t.note, amount: 0,
    accountId: t.postings[0]!.accountId, toAccountId: null, categoryId: null, splitCount: 0, fee: 0,
    tags: [...t.tags], accountDelta: null, runningBalance: null, reconciled: null,
    attachmentCount: state.attachments.filter((a) => a.txnId === t.id).length,
  }
  if (t.kind === 'transfer') {
    for (const p of t.postings) {
      if (p.categoryId) row.fee -= p.amount
      else if (p.amount < 0) row.accountId = p.accountId
      else {
        row.toAccountId = p.accountId
        row.amount = p.amount
      }
    }
  } else {
    const cats = t.postings.filter((p) => p.categoryId).map((p) => p.categoryId!)
    row.amount = rowAmount(t)
    row.splitCount = cats.length
    if (cats.length === 1) row.categoryId = cats[0]!
  }
  if (accountId && running) {
    row.accountDelta = t.postings.filter((p) => p.accountId === accountId).reduce((m, p) => m + p.amount, 0)
    row.runningBalance = running.get(t.id) ?? null
    row.reconciled = t.reconciled.includes(accountId)
  }
  return row
}

/** Saldo berjalan atas seluruh riwayat aktif akun, tidak terpengaruh filter lain. */
function runningBalances(accountId: string): Map<string, number> {
  const out = new Map<string, number>()
  let sum = 0
  const history = activeTxns().filter((t) => t.postings.some((p) => p.accountId === accountId))
  for (const t of history.sort(byNewest).reverse()) {
    for (const p of t.postings) if (p.accountId === accountId) sum += p.amount
    out.set(t.id, sum)
  }
  return out
}

/** Cermin `TxnSort::order_by` di `txn_list.rs`: tie-break selalu terbaru dulu. */
function bySort(sort: TxnSortDto | null): (a: Txn, b: Txn) => number {
  const s = sort ?? { field: 'date', desc: true }
  const dir = s.desc ? -1 : 1
  const cmp = (x: number | string, y: number | string) => (x < y ? -1 : x > y ? 1 : 0)
  if (s.field === 'date') return (a, b) => dir * (cmp(a.date, b.date) || a.seq - b.seq)
  if (s.field === 'amount') return (a, b) => dir * cmp(rowAmount(a), rowAmount(b)) || byNewest(a, b)
  const label = (t: Txn) => (payeeName(t) ?? t.note)?.toLowerCase() ?? null
  return (a, b) => {
    const x = label(a)
    const y = label(b)
    if (x === null || y === null) return (x === null ? 1 : 0) - (y === null ? 1 : 0) || byNewest(a, b)
    return dir * cmp(x, y) || byNewest(a, b)
  }
}

function listPage(q: TxnQueryDto): TxnPageDto {
  const hits = activeTxns().filter((t) => matches(t, q)).sort(bySort(q.sort))
  const running = q.accountId ? runningBalances(q.accountId) : null
  return {
    rows: hits.slice(q.offset, q.offset + q.limit).map((t) => shapeRow(t, running, q.accountId)),
    total: hits.length,
  }
}

function defaultFeeCategory(): string | null {
  const active = state.categories.filter((c) => c.kind === 'expense' && !c.archived)
  if (state.lastFeeCategoryId && active.some((c) => c.id === state.lastFeeCategoryId)) return state.lastFeeCategoryId
  return active.find((c) => SEED_FEE_NAMES.includes(c.name.toLowerCase()))?.id ?? null
}

// --- Periode budget (cermin `period.rs`) & budget (cermin app `budget.rs`) ---

type Period = { year: number; month: number }

const browserPeriod = {
  parse(key: string): Period | null {
    const m = /^(\d{4})-(\d{2})$/.exec(key)
    if (!m) return null
    const p = { year: Number(m[1]), month: Number(m[2]) }
    return p.month >= 1 && p.month <= 12 ? p : null
  },
  key: (p: Period) => `${String(p.year).padStart(4, '0')}-${String(p.month).padStart(2, '0')}`,
  shift: (p: Period, by: number): Period => {
    const n = p.year * 12 + (p.month - 1) + by
    return { year: Math.floor(n / 12), month: (n % 12) + 1 }
  },
  iso: (p: Period, day: number) => `${browserPeriod.key(p)}-${String(day).padStart(2, '0')}`,
  /** Setengah terbuka `[from, toExclusive)`; periode dinamai bulan tempat ia berakhir. */
  range(p: Period, start: number): [string, string] {
    if (start === 1) return [browserPeriod.iso(p, 1), browserPeriod.iso(browserPeriod.shift(p, 1), 1)]
    return [browserPeriod.iso(browserPeriod.shift(p, -1), start), browserPeriod.iso(p, start)]
  },
  of(date: string, start: number): Period {
    const p = { year: Number(date.slice(0, 4)), month: Number(date.slice(5, 7)) }
    return start > 1 && Number(date.slice(8, 10)) >= start ? browserPeriod.shift(p, 1) : p
  },
}

function previousDay(iso: string): string {
  const d = new Date(`${iso}T00:00:00Z`)
  d.setUTCDate(d.getUTCDate() - 1)
  return d.toISOString().slice(0, 10)
}

/** Realisasi per kategori expense dalam `[from, toEx)`; parent mencakup child (cermin `rollup_spent`). */
function spentIn(from: string, toEx: string): Map<string, number> {
  const direct = categoryTotals(from, toEx)
  const spent = new Map<string, number>()
  const add = (k: string, v: number) => spent.set(k, (spent.get(k) ?? 0) + v)
  for (const c of state.categories.filter((x) => x.kind === 'expense')) {
    const own = -(direct.get(c.id) ?? 0)
    add(c.id, own)
    if (c.parentId) add(c.parentId, own)
  }
  return spent
}

const budgetEntry = (c: string, key: string) => state.budgets.find((b) => b.categoryId === c && b.period === key)

/** Carry (K2): Σ nominal rantai rollover yang berakhir di `p - 1` dikurangi realisasi rantai. */
function carryOf(c: string, p: Period): number {
  let sum = 0
  let first: Period | null = null
  for (let q = browserPeriod.shift(p, -1); ; q = browserPeriod.shift(q, -1)) {
    const e = budgetEntry(c, browserPeriod.key(q))
    if (!e?.rollover) break
    sum += e.amount
    first = q
  }
  if (!first) return 0
  const from = browserPeriod.range(first, state.periodStartDay)[0]
  const until = browserPeriod.range(p, state.periodStartDay)[0]
  return sum - (spentIn(from, until).get(c) ?? 0)
}

/** Cermin core `budget_level`. */
function budgetLevel(available: number, spent: number): BudgetLevelDto {
  if (available > 0) return spent >= available ? 'exceeded' : spent * 100 >= available * 80 ? 'warning' : 'ok'
  return spent > 0 || available < 0 ? 'exceeded' : 'ok'
}

const spentPercent = (available: number, spent: number) =>
  available > 0 ? Math.floor((Math.max(spent, 0) * 100) / available) : null
const perDay = (remaining: number, days: number) => Math.floor(Math.max(remaining, 0) / Math.max(days, 1))

/** Hari tersisa termasuk `today` bila ada di `[from, last]` (cermin core `days_left`). */
function daysLeft(today: string, from: string, last: string): number | null {
  if (today < from || today > last) return null
  return Math.round((Date.parse(`${last}T00:00:00Z`) - Date.parse(`${today}T00:00:00Z`)) / 86_400_000) + 1
}

function budgetOverview(p: Period, today: string | null): BudgetOverviewDto {
  const [from, toEx] = browserPeriod.range(p, state.periodStartDay)
  const last = previousDay(toEx)
  const key = browserPeriod.key(p)
  const cats = state.categories.filter((c) => c.kind === 'expense')
  const spent = spentIn(from, toEx)
  const days = today === null ? null : daysLeft(today, from, last)
  const bySort = (a: CategoryDto, b: CategoryDto) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name)
  const rows: BudgetRowDto[] = []
  for (const root of cats.filter((c) => !c.parentId).sort(bySort)) {
    for (const c of [root, ...cats.filter((x) => x.parentId === root.id).sort(bySort)]) {
      const entry = budgetEntry(c.id, key)
      const budget = entry?.amount ?? null
      const s = spent.get(c.id) ?? 0
      if (c.archived && budget === null && s === 0) continue
      const carry = entry ? carryOf(c.id, p) : 0
      const available = budget === null ? null : budget + carry
      rows.push({
        categoryId: c.id, parentId: c.parentId, budget, rollover: entry?.rollover ?? false, carry, available,
        spent: s, remaining: available === null ? null : available - s,
        level: available === null ? null : budgetLevel(available, s),
        percent: available === null ? null : spentPercent(available, s),
        perDay: available === null || days === null ? null : perDay(available - s, days),
      })
    }
  }
  // Total: baris berbudget yang parent-nya tidak berbudget (aturan K2).
  const top = rows.filter((r) => r.budget !== null && !(r.parentId && rows.some((x) => x.categoryId === r.parentId && x.budget !== null)))
  const sum = (f: (r: BudgetRowDto) => number) => top.reduce((m, r) => m + f(r), 0)
  const budget = sum((r) => r.budget!)
  const carry = sum((r) => r.carry)
  const totalSpent = sum((r) => r.spent)
  const remaining = budget + carry - totalSpent
  return {
    period: key, prev: browserPeriod.key(browserPeriod.shift(p, -1)), next: browserPeriod.key(browserPeriod.shift(p, 1)),
    from, to: last, daysLeft: days, rows,
    total: { budget, carry, available: budget + carry, spent: totalSpent, remaining, perDay: days === null ? null : perDay(remaining, days) },
  }
}

// --- Peringatan budget (cermin app `budget_alerts.rs`) ---

const LEVEL_RANK: Record<BudgetLevelDto, number> = { ok: 0, warning: 1, exceeded: 2 }

function periodsOf(dates: string[]): Period[] {
  const keys = [...new Set(dates.map((d) => browserPeriod.key(browserPeriod.of(d, state.periodStartDay))))].sort()
  return keys.map((k) => browserPeriod.parse(k)!)
}

function budgetLevels(periods: Period[]): Map<string, BudgetLevelDto> {
  const out = new Map<string, BudgetLevelDto>()
  for (const p of periods) {
    for (const r of budgetOverview(p, null).rows) if (r.level) out.set(`${r.categoryId}|${browserPeriod.key(p)}`, r.level)
  }
  return out
}

/** Kategori ber-budget yang levelnya naik ke `warning`/`exceeded` dibanding `before`. */
function budgetAlerts(periods: Period[], before: Map<string, BudgetLevelDto>): BudgetAlertDto[] {
  const out: BudgetAlertDto[] = []
  for (const p of periods) {
    const period = browserPeriod.key(p)
    for (const r of budgetOverview(p, null).rows) {
      if (!r.level || r.level === 'ok') continue
      const was = before.get(`${r.categoryId}|${period}`) ?? 'ok'
      if (LEVEL_RANK[r.level] > LEVEL_RANK[was]) out.push({ categoryId: r.categoryId, period, level: r.level, percent: r.percent })
    }
  }
  return out
}

// --- Alokasi modal (cermin core `allocation.rs` & app `allocation.rs`) ---

const FULL_BP = 10_000
// Rust menghitung di i128; di sini Number cukup karena saldo ≤ MAX_AMOUNT (1e12) dan
// MAX_AMOUNT × 10000 = 1e16 < 2^53. `Math.trunc` meniru pembagian integer Rust (ke arah nol).
const mulDiv = (a: number, b: number, c: number) => Math.trunc((a * b) / c)

const checkBp = (bp: number) => Number.isInteger(bp) && bp >= 0 && bp <= FULL_BP

function bucketTree(): BucketDto[] {
  const order = (a: BucketDto, b: BucketDto) => a.sortOrder - b.sortOrder || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0)
  return state.buckets.filter((b) => !b.parentId).sort(order)
    .flatMap((root) => [root, ...state.buckets.filter((c) => c.parentId === root.id).sort(order)])
}

function validateBucket(selfId: string | null, f: BucketFieldsDto): Result<string> {
  const name = f.name.trim()
  if (!name) return fail('name_empty')
  if (f.parentId !== null) {
    if (f.parentId === selfId) return fail('allocation_depth')
    const parent = state.buckets.find((b) => b.id === f.parentId)
    if (!parent) return notFound('bucket')
    if (parent.parentId !== null) return fail('allocation_depth')
    if (selfId && state.buckets.some((b) => b.parentId === selfId)) return fail('allocation_depth')
  }
  if (f.targetAmount !== null) {
    if (!Number.isInteger(f.targetAmount) || f.targetAmount <= 0) return fail('allocation_invalid_amount')
  } else if (!checkBp(f.targetBp)) return fail('allocation_invalid_bp')
  const siblings = state.buckets.filter((b) => b.parentId === f.parentId && b.id !== selfId)
  // Σ target persen sibling ≤ 100%; bucket nominal tidak ikut dijumlah.
  const percentOf = (b: { targetBp: number; targetAmount: number | null }) => (b.targetAmount === null ? b.targetBp : 0)
  if (siblings.reduce((m, b) => m + percentOf(b), percentOf(f)) > FULL_BP) return fail('allocation_targets_exceed')
  if (siblings.some((b) => b.name.toLowerCase() === name.toLowerCase())) return fail('duplicate', 'bucket')
  return ok(name)
}

/** Kolom target seperti storage Rust: bucket nominal disimpan dengan `targetBp = 0`. */
const bucketTarget = (f: BucketFieldsDto) =>
  f.targetAmount !== null ? { targetBp: 0, targetAmount: f.targetAmount } : { targetBp: f.targetBp, targetAmount: null }

type Balance = { id: string; kind: AccountKindDto; balance: number; includeInNetWorth: boolean }

/** Cermin `level_targets` di core: per tingkat, nominal dipenuhi dulu, persen membagi sisanya;
 *  basis child = target parent. `nominal` memetakan nominal ke ruang hitung (rupiah atau bp). */
function levelTargets(buckets: BucketDto[], rootBase: number, nominal: (n: number) => number): Map<string, number> {
  const out = new Map<string, number>()
  const assign = (parentId: string | null, base: number) => {
    const siblings = buckets.filter((b) => b.parentId === parentId)
    const fixed = siblings.reduce((m, b) => m + (b.targetAmount !== null ? nominal(b.targetAmount) : 0), 0)
    const rest = Math.max(base - fixed, 0)
    for (const b of siblings) out.set(b.id, b.targetAmount !== null ? nominal(b.targetAmount) : mulDiv(rest, b.targetBp, FULL_BP))
  }
  assign(null, rootBase)
  for (const root of buckets.filter((b) => !b.parentId)) assign(root.id, out.get(root.id)!)
  return out
}

function allocationReport(accounts: Balance[]) {
  let gross = 0
  let liabilities = 0
  const counted = new Map<string, number>()
  for (const a of accounts.filter((x) => x.includeInNetWorth)) {
    if (countsAsLiability(a.kind, a.balance)) liabilities -= a.balance
    else {
      gross += a.balance
      counted.set(a.id, a.balance)
    }
  }
  const direct = new Map<string, number>()
  const mappedBp = new Map<string, number>()
  for (const m of state.mappings) {
    const balance = counted.get(m.accountId)
    if (balance === undefined) continue
    direct.set(m.bucketId, (direct.get(m.bucketId) ?? 0) + mulDiv(balance, m.shareBp, FULL_BP))
    mappedBp.set(m.accountId, (mappedBp.get(m.accountId) ?? 0) + m.shareBp)
  }
  const buckets = bucketTree()
  const bpOf = (amount: number) => (gross > 0 ? mulDiv(amount, FULL_BP, gross) : 0)
  // Dua ruang hitung dengan aturan yang sama: rupiah (basis gross) dan bp (basis 10000).
  const amountTargets = levelTargets(buckets, gross, (n) => n)
  const bpTargets = levelTargets(buckets, FULL_BP, bpOf)
  const actuals: BucketActualDto[] = buckets.map((b) => {
    const actual = (direct.get(b.id) ?? 0)
      + buckets.filter((c) => c.parentId === b.id).reduce((m, c) => m + (direct.get(c.id) ?? 0), 0)
    const targetBp = bpTargets.get(b.id) ?? 0
    const targetAmount = amountTargets.get(b.id) ?? 0
    const actualBp = bpOf(actual)
    return {
      bucketId: b.id, targetBp, targetAmount, actual, actualBp, diffBp: actualBp - targetBp,
      diffAmount: actual - targetAmount,
    }
  })
  const unallocated: UnallocatedDto[] = []
  for (const a of accounts.filter((x) => counted.has(x.id))) {
    const remainingBp = FULL_BP - (mappedBp.get(a.id) ?? 0)
    const amount = mulDiv(a.balance, remainingBp, FULL_BP)
    if (remainingBp <= 0 || amount === 0) continue
    unallocated.push({ accountId: a.id, remainingBp, amount })
  }
  const unallocatedTotal = unallocated.reduce((m, u) => m + u.amount, 0)
  const warnings: WarningDto[] = []
  const amountWarnings: AmountWarningDto[] = []
  const seen = new Set<string | null>()
  for (const b of buckets) {
    if (seen.has(b.parentId)) continue
    seen.add(b.parentId)
    const siblings = buckets.filter((s) => s.parentId === b.parentId)
    const percents = siblings.filter((s) => s.targetAmount === null)
    const totalBp = percents.reduce((m, s) => m + s.targetBp, 0)
    if (percents.length > 0 && totalBp !== FULL_BP) warnings.push({ parentId: b.parentId, totalBp })
    const totalAmount = siblings.reduce((m, s) => m + (s.targetAmount ?? 0), 0)
    const base = b.parentId ? (amountTargets.get(b.parentId) ?? 0) : gross
    if (gross > 0 && totalAmount > base) amountWarnings.push({ parentId: b.parentId, totalAmount, base })
  }
  return {
    buckets, gross, liabilities, actuals, unallocated, unallocatedTotal, unallocatedBp: bpOf(unallocatedTotal), warnings,
    amountWarnings,
  }
}

const balancesAsOf = (asOf: string | null): Balance[] =>
  state.accounts.map((a) => ({ id: a.id, kind: a.kind, balance: balanceOf(a.id, asOf), includeInNetWorth: a.includeInNetWorth }))

function allocationOverview(): AllocationOverviewDto {
  const { buckets, ...report } = allocationReport(balancesAsOf(null))
  return { buckets: buckets.map((b) => ({ ...b })), mappings: state.mappings.map((m) => ({ ...m })), ...report }
}

// --- Laporan (cermin app `reports.rs`) ---

const MAX_PERIODS = 24

/** `count` periode berakhir di `end`, terlama dulu; `count` di-clamp ke `1..=24` seperti Rust. */
function periodsEnding(end: Period, count: number): Period[] {
  const n = Math.min(Math.max(Math.trunc(count), 1), MAX_PERIODS)
  return Array.from({ length: n }, (_, i) => browserPeriod.shift(end, i - n + 1))
}

/** Σ posting per kategori dalam `[from, toEx)`; posting tanpa kategori (transfer, valuation, opening) tidak ikut. */
function categoryTotals(from: string, toEx: string): Map<string, number> {
  const totals = new Map<string, number>()
  for (const t of activeTxns()) {
    if (t.date < from || t.date >= toEx) continue
    for (const x of t.postings) if (x.categoryId) totals.set(x.categoryId, (totals.get(x.categoryId) ?? 0) + x.amount)
  }
  return totals
}

function cashflowPoint(p: Period): CashflowPointDto {
  const [from, toEx] = browserPeriod.range(p, state.periodStartDay)
  let income = 0
  let expense = 0
  for (const [c, amount] of categoryTotals(from, toEx)) {
    const kind = state.categories.find((x) => x.id === c)?.kind
    if (kind === 'income') income += amount
    else if (kind === 'expense') expense -= amount
  }
  return { period: browserPeriod.key(p), from, to: previousDay(toEx), income, expense, net: income - expense }
}

function categoryBreakdown(p: Period, kind: CategoryKindDto): CategoryAmountDto[] {
  const [from, toEx] = browserPeriod.range(p, state.periodStartDay)
  const roots = new Map<string, number>()
  for (const [c, amount] of categoryTotals(from, toEx)) {
    const cat = state.categories.find((x) => x.id === c)
    if (cat?.kind !== kind) continue
    const root = cat.parentId ?? cat.id
    roots.set(root, (roots.get(root) ?? 0) + (kind === 'income' ? amount : -amount))
  }
  const name = (c: string) => state.categories.find((x) => x.id === c)?.name ?? ''
  return [...roots]
    .filter(([, amount]) => amount > 0)
    .map(([categoryId, amount]) => ({ categoryId, amount }))
    .sort((a, b) => b.amount - a.amount || (name(a.categoryId) < name(b.categoryId) ? -1 : name(a.categoryId) > name(b.categoryId) ? 1 : 0))
}

// Port `Ledger::category_comparison`: root yang positif di salah satu periode, dengan rincian child.
function categoryComparison(p: Period, kind: CategoryKindDto): CategoryComparisonDto[] {
  const signed = (q: Period) => {
    const [from, toEx] = browserPeriod.range(q, state.periodStartDay)
    const out = new Map<string, number>()
    for (const [c, amount] of categoryTotals(from, toEx)) {
      if (state.categories.find((x) => x.id === c)?.kind === kind) out.set(c, kind === 'income' ? amount : -amount)
    }
    return out
  }
  const current = signed(p)
  const previous = signed(browserPeriod.shift(p, -1))
  const roots = new Map<string, { total: CategoryDeltaDto; children: CategoryDeltaDto[] }>()
  for (const id of new Set([...current.keys(), ...previous.keys()])) {
    const rootId = state.categories.find((x) => x.id === id)?.parentId ?? id
    let root = roots.get(rootId)
    if (!root) roots.set(rootId, (root = { total: { categoryId: rootId, amount: 0, previous: 0 }, children: [] }))
    const child = { categoryId: id, amount: current.get(id) ?? 0, previous: previous.get(id) ?? 0 }
    root.total.amount += child.amount
    root.total.previous += child.previous
    root.children.push(child)
  }
  const name = (c: string) => state.categories.find((x) => x.id === c)?.name ?? ''
  const order = (a: CategoryDeltaDto, b: CategoryDeltaDto) =>
    b.amount - a.amount || b.previous - a.previous
    || (name(a.categoryId) < name(b.categoryId) ? -1 : name(a.categoryId) > name(b.categoryId) ? 1 : 0)
  return [...roots.values()]
    .filter(({ total }) => total.amount > 0 || total.previous > 0)
    .sort((a, b) => order(a.total, b.total))
    .map(({ total, children }) => {
      let rows = children.filter((c) => c.amount !== 0 || c.previous !== 0)
      // Hanya posting langsung di root: tidak ada rincian yang perlu ditampilkan.
      if (rows.every((c) => c.categoryId === total.categoryId)) rows = []
      return { ...total, children: rows.sort(order) }
    })
}

// Port `Ledger::category_trend`: kategori + child-nya per periode, bertanda sesuai jenis.
function categoryTrendPoint(p: Period, ids: Set<string>, kind: CategoryKindDto): CategoryTrendPointDto {
  const [from, toEx] = browserPeriod.range(p, state.periodStartDay)
  let amount = 0
  for (const [c, total] of categoryTotals(from, toEx)) if (ids.has(c)) amount += kind === 'income' ? total : -total
  return { period: browserPeriod.key(p), from, to: previousDay(toEx), amount }
}

function netWorthPoint(p: Period): NetWorthPointDto {
  const date = previousDay(browserPeriod.range(p, state.periodStartDay)[1])
  let assets = 0
  let liabilities = 0
  for (const b of balancesAsOf(date)) {
    if (!b.includeInNetWorth) continue
    if (countsAsLiability(b.kind, b.balance)) liabilities -= b.balance
    else assets += b.balance
  }
  return { period: browserPeriod.key(p), date, assets, liabilities, net: assets - liabilities }
}

// Backup di browser = snapshot state di memori; nama mengikuti pola Rust (`{kind}-{stamp}Z.db`).
type Snapshot = { info: BackupDto; state: typeof state }
const backups: Snapshot[] = []

function snapshot(kind: BackupKindDto): BackupDto {
  const now = new Date()
  const stamp = now.toISOString().replace(/[-:]/g, '').replace(/\.\d{3}/, '')
  const base = `${kind.replace('_', '-')}-${stamp}`
  let fileName = `${base}.db`
  for (let n = 2; backups.some((b) => b.info.fileName === fileName); n++) fileName = `${base}-${n}.db`
  const info: BackupDto = { fileName, kind, createdAt: now.toISOString(), sizeBytes: 0 }
  backups.unshift({ info, state: structuredClone(state) })
  return info
}

// --- Inbox email capture: port `crates/app/src/capture.rs` + `find_duplicate_txns`. ---

const MAX_DUPLICATES = 5
const dayNumber = (iso: string) => Date.parse(`${iso}T00:00:00Z`) / 86_400_000
const byCaptureNewest = (a: Capture, b: Capture) => b.date.localeCompare(a.date) || b.id.localeCompare(a.id)

/** Nominal & akun tampilan kandidat duplikat (K4). */
function duplicateOf(t: Txn): DuplicateTxnDto {
  const legs = t.postings.filter((p) => !p.categoryId)
  const transfer = t.kind === 'transfer'
  return {
    id: t.id, date: t.date, payee: payeeName(t),
    amount: transfer ? Math.max(...legs.map((p) => p.amount)) : Math.abs(t.postings.reduce((m, p) => m + p.amount, 0)),
    accountId: (transfer ? legs.find((p) => p.amount < 0) : t.postings[0])?.accountId ?? '',
  }
}

function duplicatesOf(c: Capture): DuplicateTxnDto[] {
  const day = dayNumber(c.date)
  return activeTxns()
    .filter((t) => t.kind === c.kind && Math.abs(dayNumber(t.date) - day) <= 1)
    .map((t) => ({ t, d: duplicateOf(t) }))
    .filter(({ t, d }) => d.amount === c.amount && (c.accountId === null || t.postings.some((p) => p.accountId === c.accountId)))
    .sort((a, b) => Math.abs(dayNumber(a.t.date) - day) - Math.abs(dayNumber(b.t.date) - day)
      || a.t.date.localeCompare(b.t.date) || a.t.id.localeCompare(b.t.id))
    .slice(0, MAX_DUPLICATES)
    .map(({ d }) => d)
}

function pendingCapture(i: string): Result<Capture> {
  const c = state.captures.find((x) => x.id === i)
  if (!c) return notFound('email capture')
  return c.status === 'pending' ? ok(c) : fail('capture_not_pending')
}

const capturesWith = (status: Capture['status']) => state.captures.filter((c) => c.status === status)

/** Hook test/e2e: isi staging seolah hasil sync (K14). */
export function __seedEmailInbox(seed: { captures?: BrowserCaptureSeed[]; unparsed?: BrowserUnparsedSeed[] }): void {
  for (const c of seed.captures ?? []) state.captures.push({ ...c, id: c.id ?? id(), status: 'pending', txnId: null })
  for (const u of seed.unparsed ?? []) state.unparsed.unshift({ ...u })
}

// --- Rule email: port `crates/app/src/email_rules.rs`. ---

/** Offset lokal browser dalam menit (Rust: `state.email.local_offset`). */
const localOffset = () => -new Date().getTimezoneOffset()
const optPattern = (s: string | null) => (s === null || s.trim() === '' ? null : s)
const byRuleOrder = (a: EmailRule, b: EmailRule) =>
  b.priority - a.priority || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0) || (a.id < b.id ? -1 : 1)
const activeEmailRules = () => state.emailRules.filter((r) => !r.deleted && r.enabled).sort(byRuleOrder)

/** `normalized()`: nama & pengirim di-trim; pola opsional kosong → null. */
function normalizedRule(i: EmailRuleInputDto): EmailRuleInputDto {
  return {
    ...i, name: i.name.trim(), senderPattern: i.senderPattern.trim(), subjectRegex: optPattern(i.subjectRegex),
    payeeRegex: optPattern(i.payeeRegex), dateRegex: optPattern(i.dateRegex), dateFormat: optPattern(i.dateFormat),
  }
}

/** Email tersimpan sebagai pesan rule engine; `null` = raw tidak terbaca saat sync. */
function storedMessage(u: UnparsedEmailDto): Mail | null {
  if (u.sender === '') return null
  const at = u.receivedAt === null ? NaN : Date.parse(u.receivedAt)
  return { from: u.sender, subject: u.subject, sentAt: Number.isNaN(at) ? null : Math.floor(at / 1000), text: u.snippet }
}

function activeRef<T extends { archived: boolean }>(x: T | undefined, entity: string): Result<T> {
  if (!x) return notFound(entity)
  return x.archived ? fail('archived', entity) : ok(x)
}

/** `check` (K3): pola lalu referensi akun/kategori. */
function checkEmailRule(r: EmailRuleInputDto): Result<null> {
  const field = validate(r)
  if (field !== null) return fail('email_rule_invalid', field)
  if (r.accountId !== null) {
    const a = activeRef(state.accounts.find((x) => x.id === r.accountId), 'account')
    if (a.status === 'error') return a
  }
  const transfer = r.kind === 'transfer_out' || r.kind === 'transfer_in'
  if (r.counterAccountId !== null) {
    if (!transfer) return fail('email_rule_invalid', 'counter_account')
    if (r.accountId === r.counterAccountId) return fail('txn_transfer_same_account')
    const a = activeRef(state.accounts.find((x) => x.id === r.counterAccountId), 'account')
    if (a.status === 'error') return a
  }
  if (r.defaultCategoryId !== null) {
    const c = activeRef(state.categories.find((x) => x.id === r.defaultCategoryId), 'category')
    if (c.status === 'error') return c
    if (transfer) return fail('txn_unexpected_category')
    if (c.data.kind !== r.kind) return fail('txn_wrong_category_kind')
  }
  return ok(null)
}

/** `reparse_in` (K6/K7): unparsed yang lolos rule aktif pindah ke capture pending. */
function reparse(): ReparseSummaryDto {
  const rules = activeEmailRules()
  const summary = { emails: 0, items: 0 }
  for (const u of [...state.unparsed]) {
    const msg = storedMessage(u)
    if (msg === null) continue
    const hit = apply(rules, msg, localOffset())
    if (hit === null) continue
    const rule = rules[hit.rule]!
    for (const item of hit.items) {
      state.captures.push({
        id: id(), kind: 'expense', date: item.date, amount: item.amount, payee: item.payee, accountId: null,
        toAccountId: null, categoryId: null, ruleName: rule.name, snippet: u.snippet, status: 'pending', txnId: null,
        ruleId: rule.id,
      })
      summary.items += 1
    }
    state.unparsed.splice(state.unparsed.indexOf(u), 1)
    summary.emails += 1
  }
  return summary
}

/** `draft_of`: kind/akun/kategori dari rule (termasuk yang dihapus) dan saran payee. */
function captureDraft(c: Capture): Omit<CaptureDto, 'duplicates'> {
  const { status: _, txnId: __, ruleId, ...dto } = c
  const rule = state.emailRules.find((r) => r.id === ruleId)
  if (!rule) return dto
  const p = c.payee === null ? undefined : state.payees.find((x) => sameName(x.name, c.payee!))
  const last = p ? activeTxns().filter((t) => t.payeeId === p.id).sort(byNewest)[0] : undefined
  const row = last ? shapeRow(last, null, null) : null
  const payeeCategory = state.categories.find((x) => x.id === p?.defaultCategoryId && !x.archived)?.id ?? row?.categoryId ?? null
  const base = { ...dto, ruleName: rule.name }
  switch (rule.kind) {
    case 'income':
    case 'expense':
      return {
        ...base, kind: rule.kind, accountId: rule.accountId ?? row?.accountId ?? null, toAccountId: null,
        categoryId: rule.defaultCategoryId ?? payeeCategory,
      }
    case 'transfer_out':
      return { ...base, kind: 'transfer', accountId: rule.accountId, toAccountId: rule.counterAccountId, categoryId: null }
    case 'transfer_in':
      return { ...base, kind: 'transfer', accountId: rule.counterAccountId, toAccountId: rule.accountId, categoryId: null }
  }
}

function findEmailRule(i: string, deleted: boolean): Result<EmailRule> {
  const r = state.emailRules.find((x) => x.id === i && x.deleted === deleted)
  return r ? ok(r) : notFound('email rule')
}

function unparsedMessage(messageId: string): Result<Mail> {
  const u = state.unparsed.find((x) => x.messageId === messageId)
  const msg = u ? storedMessage(u) : null
  return msg ? ok(msg) : notFound('email')
}

/** Salin rule bawaan sekali (K11), dipanggil saat akun email disimpan. */
export function __seedDefaultEmailRules(): void {
  if (state.emailRulesSeeded) return
  for (const r of DEFAULT_RULES) state.emailRules.push({ ...r, id: id(), deleted: false })
  state.emailRulesSeeded = true
}

export const browserLedger = {
  startupReport: async (): Promise<StartupReportDto> => ({ uncleanShutdown: false, integrity: 'skipped' }),
  appReady: async (): Promise<void> => {},
  listBackups: async (): Promise<Result<BackupDto[]>> => ok(backups.map((b) => ({ ...b.info }))),
  backupNow: async (): Promise<Result<string | null>> => {
    snapshot('auto')
    return ok('browser://backup')
  },
  restoreBackup: async (source: BackupSourceDto, _srcRecoveryKey: string | null): Promise<Result<null>> => {
    const src = source.kind === 'named' ? backups.find((b) => b.info.fileName === source.fileName) : undefined
    if (!src) return fail('backup_invalid')
    snapshot('pre_restore')
    Object.assign(state, structuredClone(src.state))
    return ok(null)
  },
  // Mode browser tidak punya dialog file.
  pickBackupFile: async (): Promise<Result<string | null>> => ok(null),
  exportJson: async (): Promise<Result<string | null>> => ok('browser://export.json'),
  exportCsv: async (): Promise<Result<string | null>> => ok('browser://nabungo-export'),
  getOnboardingStatus: async () => ok(state.onboarded),
  completeOnboarding: async (input: OnboardingDto): Promise<Result<string>> => {
    if (state.onboarded) return { status: 'error', error: { code: 'already_onboarded', entity: null } }
    state.periodStartDay = input.periodStartDay
    seedCategories()
    const created = await browserLedger.createAccount(input.accountKind, input.account)
    state.onboarded = true
    return created
  },
  getPeriodStartDay: async () => ok(state.periodStartDay),
  setPeriodStartDay: async (day: number) => {
    state.periodStartDay = day
    return ok(null)
  },

  accountsOverview: async () => ok(overview()),
  createAccount: async (kind: AccountKindDto, fields: AccountFieldsDto): Promise<Result<string>> => {
    const siblings = state.accounts.filter((a) => a.groupId === fields.groupId)
    const a = toAccount(kind, fields, { id: id(), sortOrder: nextSort(siblings), archived: false })
    state.accounts.push(a)
    syncOpening(a)
    return ok(a.id)
  },
  updateAccount: async (i: string, fields: AccountFieldsDto): Promise<Result<null>> => {
    const n = state.accounts.findIndex((a) => a.id === i)
    const cur = state.accounts[n]
    if (!cur) return notFound('account')
    state.accounts[n] = toAccount(cur.kind, fields, cur)
    syncOpening(state.accounts[n]!)
    return ok(null)
  },
  archiveAccount: async (i: string) => setArchived(state.accounts, i, true, 'account'),
  unarchiveAccount: async (i: string) => setArchived(state.accounts, i, false, 'account'),
  deleteAccount: async (i: string) => {
    state.accounts = state.accounts.filter((a) => a.id !== i)
    state.mappings = state.mappings.filter((m) => m.accountId !== i)
    return ok(null)
  },
  reorderAccounts: async (groupId: string | null, ids: string[]) => {
    for (const a of state.accounts) if (ids.includes(a.id)) a.groupId = groupId
    reorder(state.accounts, ids)
    return ok(null)
  },
  createAccountGroup: async (name: string): Promise<Result<string>> => {
    const g = { id: id(), name: name.trim(), sortOrder: nextSort(state.groups) }
    state.groups.push(g)
    return ok(g.id)
  },
  renameAccountGroup: async (i: string, name: string): Promise<Result<null>> => {
    const g = state.groups.find((x) => x.id === i)
    if (!g) return notFound('account_group')
    g.name = name.trim()
    return ok(null)
  },
  deleteAccountGroup: async (i: string) => {
    state.groups = state.groups.filter((g) => g.id !== i)
    for (const a of state.accounts) if (a.groupId === i) a.groupId = null
    return ok(null)
  },
  reorderAccountGroups: async (ids: string[]) => {
    reorder(state.groups, ids)
    return ok(null)
  },

  listCategories: async () => ok(state.categories.map((c) => ({ ...c }))),
  createCategory: async (kind: CategoryKindDto, fields: CategoryFieldsDto) => ok(insertCategory(kind, fields)),
  updateCategory: async (i: string, fields: CategoryFieldsDto): Promise<Result<null>> => {
    const c = state.categories.find((x) => x.id === i)
    if (!c) return notFound('category')
    Object.assign(c, { name: fields.name.trim(), parentId: fields.parentId, icon: fields.icon, color: fields.color })
    return ok(null)
  },
  archiveCategory: async (i: string) => {
    for (const c of state.categories) if (c.parentId === i) c.archived = true
    return setArchived(state.categories, i, true, 'category')
  },
  unarchiveCategory: async (i: string) => setArchived(state.categories, i, false, 'category'),
  deleteCategory: async (i: string) => {
    state.categories = state.categories.filter((c) => c.id !== i)
    return ok(null)
  },
  mergeCategory: async (source: string, _target: string) => {
    state.categories = state.categories.filter((c) => c.id !== source)
    return ok(null)
  },
  createTxn: async (form: TxnFormDto): Promise<Result<SavedTxnDto>> => {
    const periods = periodsOf([form.date])
    const before = budgetLevels(periods)
    const r = writeTxn(null, form)
    return r.status === 'ok' ? ok({ id: r.data, alerts: budgetAlerts(periods, before) }) : r
  },
  updateTxn: async (i: string, form: TxnFormDto): Promise<Result<BudgetAlertDto[]>> => {
    const t = activeTxns().find((x) => x.id === i)
    if (!t) return notFound('transaction')
    if (t.kind === 'opening') return fail('opening_not_allowed')
    if (t.kind === 'valuation' || t.kind === 'adjustment') return fail('txn_not_editable')
    const periods = periodsOf([t.date, form.date])
    const before = budgetLevels(periods)
    const r = writeTxn(t, form)
    return r.status === 'ok' ? ok(budgetAlerts(periods, before)) : r
  },
  listRules: async (): Promise<Result<RuleDto[]>> => {
    const today = todayIso()
    // Seperti `post_auto_due` saat startup/scheduler; jadwal yang gagal dilewati.
    for (const r of state.rules) if (r.autoPost) postAllDue(r, today)
    return ok([...state.rules].sort((a, b) => Number(a.ended) - Number(b.ended) || a.next.localeCompare(b.next) || a.id.localeCompare(b.id)).map((r) => ruleDto(r, today)))
  },
  createRule: async (input: RuleInputDto): Promise<Result<string>> => {
    const c = checkRule(input)
    if (c.status === 'error') return c
    const r: Rule = {
      id: id(), form: { ...input.form, tags: [...input.form.tags] }, recurrence: { ...input.recurrence },
      autoPost: input.autoPost, anchor: input.form.date, next: input.form.date, ended: false,
    }
    state.rules.push(r)
    return ok(r.id)
  },
  updateRule: async (i: string, input: RuleInputDto): Promise<Result<null>> => {
    const r = state.rules.find((x) => x.id === i)
    if (!r) return notFound('recurring_rule')
    const c = checkRule(input)
    if (c.status === 'error') return c
    // Seperti `update_rule`: anchor lama dipertahankan di tanggal ter-clamp; jadwal selesai
    // mulai dari kejadian sesudah yang terakhir ditangani.
    const anchor = keepDay(input.recurrence, input.form.date, r.recurrence, r.anchor)
    const next = r.ended && input.form.date === r.next ? nextAfter(input.recurrence, anchor, r.next) : input.form.date
    Object.assign(r, {
      form: { ...input.form, tags: [...input.form.tags] }, recurrence: { ...input.recurrence },
      autoPost: input.autoPost, anchor, next: next ?? input.form.date, ended: next === null,
    })
    return ok(null)
  },
  deleteRule: async (i: string): Promise<Result<null>> => {
    if (!state.rules.some((x) => x.id === i)) return notFound('recurring_rule')
    state.rules = state.rules.filter((x) => x.id !== i)
    return ok(null)
  },
  confirmOccurrence: async (i: string, date: string, form: TxnFormDto | null): Promise<Result<SavedTxnDto>> => {
    const p = pendingRule(i, date)
    if (p.status === 'error') return p
    const periods = periodsOf([form?.date ?? date])
    const before = budgetLevels(periods)
    const w = postOccurrence(p.data, date, form)
    return w.status === 'ok' ? ok({ id: w.data, alerts: budgetAlerts(periods, before) }) : w
  },
  skipOccurrence: async (i: string, date: string): Promise<Result<null>> => {
    const p = pendingRule(i, date)
    if (p.status === 'error') return p
    const next = nextAfter(p.data.recurrence, p.data.anchor, date)
    if (next === null) p.data.ended = true
    else p.data.next = next
    return ok(null)
  },
  confirmAllDue: async (i: string): Promise<Result<number>> => {
    const r = state.rules.find((x) => x.id === i)
    if (!r) return notFound('recurring_rule')
    return postAllDue(r, todayIso())
  },
  previewRecurrence: async (recurrence: RecurrenceDto, start: string, count: number): Promise<Result<string[]>> => {
    if (recurrence.interval < 1 || recurrence.interval > MAX_INTERVAL) return fail('recurrence_interval')
    if (recurrence.until !== null && recurrence.until < start) return fail('recurrence_until')
    return ok(occurrences(recurrence, start, Math.min(count, 12)))
  },
  getTxn: async (i: string): Promise<Result<TxnDetailDto>> => {
    const t = activeTxns().find((x) => x.id === i)
    if (!t) return notFound('transaction')
    return ok({
      id: t.id, kind: t.kind, date: t.date, payee: payeeName(t), note: t.note, tags: [...t.tags],
      postings: t.postings.map((p) => ({ ...p })),
    })
  },
  listTxns: async (query: TxnQueryDto) => ok(listPage(query)),
  deleteTxn: async (i: string): Promise<Result<null>> => {
    const t = activeTxns().find((x) => x.id === i)
    if (!t) return notFound('transaction')
    t.deleted = true
    return ok(null)
  },
  undoDeleteTxn: async (i: string): Promise<Result<null>> => {
    const t = state.txns.find((x) => x.id === i && x.deleted)
    if (!t) return notFound('transaction')
    t.deleted = false
    return ok(null)
  },
  payeeSuggestion: async (name: string): Promise<Result<PayeeSuggestionDto | null>> => {
    const p = state.payees.find((x) => sameName(x.name, name.trim()))
    if (!p) return ok(null)
    const last = activeTxns().filter((t) => t.payeeId === p.id).sort(byNewest)[0]
    const row = last ? shapeRow(last, null, null) : null
    const def = state.categories.find((c) => c.id === p.defaultCategoryId && !c.archived)
    return ok({ accountId: row?.accountId ?? null, categoryId: def?.id ?? row?.categoryId ?? null })
  },
  searchPayees: async (query: string) => {
    const q = query.trim().toLowerCase()
    return ok(state.payees.filter((p) => p.name.toLowerCase().includes(q)).map((p) => p.name).slice(0, 8))
  },
  listTags: async () => ok([...state.tags].sort(byName).map((t) => t.name)),
  listTagUsage: async (): Promise<Result<TagUsageDto[]>> => {
    const active = activeTxns()
    return ok([...state.tags].sort(byName).map((t) => ({
      id: t.id, name: t.name, txnCount: active.filter((x) => x.tags.includes(t.name)).length,
    })))
  },
  createTag: async (name: string): Promise<Result<string>> => {
    const n = tagName(name)
    if (n.status === 'error') return n
    if (state.tags.some((t) => sameName(t.name, n.data))) return fail('duplicate', 'tag')
    const t = { id: id(), name: n.data }
    state.tags.push(t)
    return ok(t.id)
  },
  renameTag: async (i: string, name: string): Promise<Result<null>> => {
    const n = tagName(name)
    if (n.status === 'error') return n
    const t = state.tags.find((x) => x.id === i)
    if (!t) return notFound('tag')
    if (state.tags.some((x) => x.id !== i && sameName(x.name, n.data))) return fail('duplicate', 'tag')
    retag(t.name, n.data)
    t.name = n.data
    return ok(null)
  },
  mergeTag: async (source: string, target: string): Promise<Result<null>> => {
    const s = state.tags.find((x) => x.id === source)
    const t = state.tags.find((x) => x.id === target)
    if (!s || !t) return notFound('tag')
    if (source === target) return fail('merge_into_self')
    retag(s.name, t.name)
    state.tags = state.tags.filter((x) => x.id !== source)
    return ok(null)
  },
  deleteTag: async (i: string): Promise<Result<null>> => {
    const t = state.tags.find((x) => x.id === i)
    if (!t) return notFound('tag')
    retag(t.name, null)
    state.tags = state.tags.filter((x) => x.id !== i)
    return ok(null)
  },
  listPayeeUsage: async (): Promise<Result<PayeeUsageDto[]>> => {
    const active = activeTxns()
    return ok([...state.payees].sort(byName).map((p) => {
      const used = active.filter((t) => t.payeeId === p.id)
      const lastDate = used.reduce<string | null>((m, t) => (m === null || t.date > m ? t.date : m), null)
      return { id: p.id, name: p.name, defaultCategoryId: p.defaultCategoryId, txnCount: used.length, lastDate }
    }))
  },
  createPayee: async (name: string, defaultCategoryId: string | null): Promise<Result<string>> => {
    const n = catalogName(name)
    if (n.status === 'error') return n
    const c = checkDefaultCategory(defaultCategoryId)
    if (c.status === 'error') return c
    if (state.payees.some((p) => sameName(p.name, n.data))) return fail('duplicate', 'payee')
    const p = { id: id(), name: n.data, defaultCategoryId }
    state.payees.push(p)
    return ok(p.id)
  },
  updatePayee: async (i: string, name: string, defaultCategoryId: string | null): Promise<Result<null>> => {
    const n = catalogName(name)
    if (n.status === 'error') return n
    const p = state.payees.find((x) => x.id === i)
    if (!p) return notFound('payee')
    // Default lama yang sudah diarsip boleh tetap; hanya nilai baru yang divalidasi (sama dengan Rust).
    if (p.defaultCategoryId !== defaultCategoryId) {
      const c = checkDefaultCategory(defaultCategoryId)
      if (c.status === 'error') return c
    }
    if (state.payees.some((x) => x.id !== i && sameName(x.name, n.data))) return fail('duplicate', 'payee')
    Object.assign(p, { name: n.data, defaultCategoryId })
    return ok(null)
  },
  // Termasuk transaksi terhapus, agar undo memakai payee hasil gabung (sama dengan Rust).
  mergePayee: async (source: string, target: string): Promise<Result<null>> => {
    if (!state.payees.some((p) => p.id === source) || !state.payees.some((p) => p.id === target)) return notFound('payee')
    if (source === target) return fail('merge_into_self')
    for (const t of state.txns) if (t.payeeId === source) t.payeeId = target
    state.payees = state.payees.filter((p) => p.id !== source)
    return ok(null)
  },
  deletePayee: async (i: string): Promise<Result<null>> => {
    if (!state.payees.some((p) => p.id === i)) return notFound('payee')
    for (const t of state.txns) if (t.payeeId === i) t.payeeId = null
    state.payees = state.payees.filter((p) => p.id !== i)
    return ok(null)
  },
  // Tanpa filesystem: setiap panggilan menambah satu struk contoh.
  addAttachments: async (txnId: string): Promise<Result<AttachmentDto[]>> => {
    if (!activeTxns().some((t) => t.id === txnId)) return notFound('transaction')
    const a = { id: id(), txnId, fileName: 'struk-contoh.jpg', mime: 'image/jpeg', sizeBytes: 1234, missing: false }
    state.attachments.push(a)
    return ok([attachmentDto(a)])
  },
  listAttachments: async (txnId: string): Promise<Result<AttachmentDto[]>> =>
    ok(state.attachments.filter((a) => a.txnId === txnId).map(attachmentDto)),
  removeAttachment: async (i: string): Promise<Result<null>> => {
    if (!state.attachments.some((a) => a.id === i)) return notFound('attachment')
    state.attachments = state.attachments.filter((a) => a.id !== i)
    return ok(null)
  },
  openAttachment: async (i: string): Promise<Result<null>> =>
    state.attachments.some((a) => a.id === i) ? ok(null) : notFound('attachment'),
  txnFormDefaults: async (): Promise<Result<TxnFormDefaultsDto>> => {
    const last = activeTxns().filter((t) => t.kind !== 'opening').sort((a, b) => b.seq - a.seq)[0]
    return ok({ lastAccountId: last ? shapeRow(last, null, null).accountId : null, feeCategoryId: defaultFeeCategory() })
  },
  periodOf: async (date: string): Promise<Result<string>> =>
    ok(browserPeriod.key(browserPeriod.of(date, state.periodStartDay))),
  budgetOverview: async (period: string): Promise<Result<BudgetOverviewDto>> => {
    const p = browserPeriod.parse(period)
    return p ? ok(budgetOverview(p, todayIso())) : fail('invalid_period')
  },
  setBudget: async (categoryId: string, period: string, amount: number | null): Promise<Result<null>> => {
    if (!browserPeriod.parse(period)) return fail('invalid_period')
    const c = state.categories.find((x) => x.id === categoryId)
    if (!c) return notFound('category')
    if (amount !== null && c.archived) return fail('archived', 'category')
    if (c.kind !== 'expense') return fail('budget_income_category')
    if (amount !== null && amount < 0) return fail('budget_negative')
    if (amount !== null && amount > MAX_AMOUNT) return fail('budget_too_large')
    const existing = budgetEntry(categoryId, period)
    if (amount === null) state.budgets = state.budgets.filter((b) => b !== existing)
    else if (existing) existing.amount = amount
    else state.budgets.push({ categoryId, period, amount, rollover: false })
    return ok(null)
  },
  setBudgetRollover: async (categoryId: string, period: string, on: boolean): Promise<Result<null>> => {
    if (!browserPeriod.parse(period)) return fail('invalid_period')
    const c = state.categories.find((x) => x.id === categoryId)
    if (!c) return notFound('category')
    if (on && c.archived) return fail('archived', 'category')
    if (c.kind !== 'expense') return fail('budget_income_category')
    const b = budgetEntry(categoryId, period)
    if (!b) return notFound('budget')
    b.rollover = on
    return ok(null)
  },
  copyBudgets: async (period: string): Promise<Result<number>> => {
    const p = browserPeriod.parse(period)
    if (!p) return fail('invalid_period')
    const prev = browserPeriod.key(browserPeriod.shift(p, -1))
    let copied = 0
    for (const b of state.budgets.filter((x) => x.period === prev)) {
      const c = state.categories.find((x) => x.id === b.categoryId)
      if (!c || c.archived || c.kind !== 'expense') continue
      if (state.budgets.some((x) => x.categoryId === b.categoryId && x.period === period)) continue
      state.budgets.push({ categoryId: b.categoryId, period, amount: b.amount, rollover: b.rollover })
      copied++
    }
    return ok(copied)
  },
  // Seperti `Ledger::record_valuation`: yang disimpan selisih terhadap saldo per akhir `date`.
  recordValuation: async (accountId: string, date: string, value: number): Promise<Result<string>> => {
    if (Math.abs(value) > MAX_AMOUNT) return fail('txn_amount_too_large')
    const a = state.accounts.find((x) => x.id === accountId)
    if (!a) return notFound('account')
    if (a.archived) return fail('archived', 'account')
    const change = value - balanceOf(accountId, date)
    if (change === 0) return fail('valuation_unchanged')
    if (Math.abs(change) > MAX_AMOUNT) return fail('txn_amount_too_large')
    if (a.kind !== 'investment') return fail('txn_valuation_non_investment')
    const t: Txn = {
      id: id(), seq: ++seq, kind: 'valuation', date, payeeId: null, note: null, tags: [], reconciled: [], deleted: false,
      postings: [{ accountId, categoryId: null, amount: change, memo: null }],
    }
    state.txns.push(t)
    return ok(t.id)
  },
  // Seperti `Ledger::reconcile_account`: selisih jadi transaksi `adjustment`, lalu posting ≤ `date` ditandai.
  reconcileAccount: async (accountId: string, date: string, balance: number): Promise<Result<ReconciliationDto>> => {
    if (Math.abs(balance) > MAX_AMOUNT) return fail('txn_amount_too_large')
    const a = state.accounts.find((x) => x.id === accountId)
    if (!a) return notFound('account')
    if (a.archived) return fail('archived', 'account')
    const difference = balance - balanceOf(accountId, date)
    if (Math.abs(difference) > MAX_AMOUNT) return fail('txn_amount_too_large')
    let adjustmentId: string | null = null
    if (difference !== 0) {
      adjustmentId = id()
      state.txns.push({
        id: adjustmentId, seq: ++seq, kind: 'adjustment', date, payeeId: null, note: null, tags: [], reconciled: [], deleted: false,
        postings: [{ accountId, categoryId: null, amount: difference, memo: null }],
      })
    }
    let marked = 0
    for (const t of activeTxns()) {
      if (t.date > date) continue
      const n = t.postings.filter((p) => p.accountId === accountId).length
      if (n === 0) continue
      if (!t.reconciled.includes(accountId)) t.reconciled.push(accountId)
      marked += n
    }
    return ok({ adjustmentId, difference, marked })
  },
  accountBalanceAsOf: async (accountId: string, date: string): Promise<Result<number>> => {
    if (!state.accounts.some((x) => x.id === accountId)) return notFound('account')
    return ok(balanceOf(accountId, date))
  },
  listContacts: async (): Promise<Result<ContactDto[]>> =>
    ok([...state.contacts].sort((x, y) => (x.name.toLowerCase() < y.name.toLowerCase() ? -1 : x.name.toLowerCase() > y.name.toLowerCase() ? 1 : 0))),
  valuationHistory: async (accountId: string): Promise<Result<ValuationPointDto[]>> => {
    return ok(activeTxns()
      .filter((t) => t.kind === 'valuation' && t.postings[0]?.accountId === accountId)
      .sort(byNewest)
      .map((t) => ({ txnId: t.id, date: t.date, change: t.postings[0]!.amount, value: balanceOf(accountId, t.date) })))
  },

  listAllocation: async (): Promise<Result<AllocationOverviewDto>> => ok(allocationOverview()),
  createBucket: async (fields: BucketFieldsDto): Promise<Result<string>> => {
    const v = validateBucket(null, fields)
    if (v.status === 'error') return v
    const b: BucketDto = {
      id: id(), parentId: fields.parentId, name: v.data, ...bucketTarget(fields), color: fields.color,
      sortOrder: nextSort(state.buckets.filter((x) => x.parentId === fields.parentId)),
    }
    state.buckets.push(b)
    return ok(b.id)
  },
  updateBucket: async (i: string, fields: BucketFieldsDto): Promise<Result<null>> => {
    if (!fields.name.trim()) return fail('name_empty')
    const b = state.buckets.find((x) => x.id === i)
    if (!b) return notFound('bucket')
    const v = validateBucket(i, fields)
    if (v.status === 'error') return v
    const sortOrder = b.parentId === fields.parentId
      ? b.sortOrder
      : nextSort(state.buckets.filter((x) => x.parentId === fields.parentId))
    Object.assign(b, { name: v.data, parentId: fields.parentId, ...bucketTarget(fields), color: fields.color, sortOrder })
    return ok(null)
  },
  deleteBucket: async (i: string): Promise<Result<null>> => {
    if (!state.buckets.some((b) => b.id === i)) return notFound('bucket')
    if (state.buckets.some((b) => b.parentId === i)) return fail('in_use', 'bucket')
    state.buckets = state.buckets.filter((b) => b.id !== i)
    state.mappings = state.mappings.filter((m) => m.bucketId !== i)
    return ok(null)
  },
  setAccountAllocation: async (accountId: string, shares: ShareDto[]): Promise<Result<null>> => {
    const a = state.accounts.find((x) => x.id === accountId)
    if (!a) return notFound('account')
    if (isLiability(a.kind)) return fail('allocation_liability')
    const seen = new Set<string>()
    let total = 0
    for (const s of shares) {
      if (!checkBp(s.shareBp)) return fail('allocation_invalid_bp')
      if (seen.has(s.bucketId)) return fail('allocation_duplicate_bucket')
      seen.add(s.bucketId)
      total += s.shareBp
    }
    if (total > FULL_BP) return fail('allocation_shares_exceed')
    if (shares.some((s) => !state.buckets.some((b) => b.id === s.bucketId))) return notFound('bucket')
    state.mappings = state.mappings
      .filter((m) => m.accountId !== accountId)
      .concat(shares.map((s) => ({ accountId, bucketId: s.bucketId, shareBp: s.shareBp })))
    return ok(null)
  },
  // Seperti `Ledger::allocation_history`: saldo per hari terakhir tiap periode, terlama dulu.
  allocationHistory: async (endPeriod: string, count: number): Promise<Result<AllocationPointDto[]>> => {
    const end = browserPeriod.parse(endPeriod)
    if (!end) return fail('invalid_period')
    const points: AllocationPointDto[] = []
    for (const p of periodsEnding(end, count)) {
      const date = previousDay(browserPeriod.range(p, state.periodStartDay)[1])
      const r = allocationReport(balancesAsOf(date))
      points.push({
        period: browserPeriod.key(p), date, gross: r.gross,
        buckets: r.buckets.flatMap((b, n) => (b.parentId ? [] : [{ bucketId: b.id, actualBp: r.actuals[n]!.actualBp }])),
      })
    }
    return ok(points)
  },
  cashflowReport: async (endPeriod: string, count: number): Promise<Result<CashflowPointDto[]>> => {
    const end = browserPeriod.parse(endPeriod)
    return end ? ok(periodsEnding(end, count).map(cashflowPoint)) : fail('invalid_period')
  },
  categoryBreakdown: async (period: string, kind: CategoryKindDto): Promise<Result<CategoryAmountDto[]>> => {
    const p = browserPeriod.parse(period)
    return p ? ok(categoryBreakdown(p, kind)) : fail('invalid_period')
  },
  categoryComparison: async (period: string, kind: CategoryKindDto): Promise<Result<CategoryComparisonDto[]>> => {
    const p = browserPeriod.parse(period)
    return p ? ok(categoryComparison(p, kind)) : fail('invalid_period')
  },
  categoryTrend: async (endPeriod: string, count: number, categoryId: string): Promise<Result<CategoryTrendPointDto[]>> => {
    const end = browserPeriod.parse(endPeriod)
    if (!end) return fail('invalid_period')
    const cat = state.categories.find((x) => x.id === categoryId)
    if (!cat) return notFound('category')
    const ids = new Set(state.categories.filter((x) => x.id === categoryId || x.parentId === categoryId).map((x) => x.id))
    return ok(periodsEnding(end, count).map((p) => categoryTrendPoint(p, ids, cat.kind)))
  },
  netWorthHistory: async (endPeriod: string, count: number): Promise<Result<NetWorthPointDto[]>> => {
    const end = browserPeriod.parse(endPeriod)
    return end ? ok(periodsEnding(end, count).map(netWorthPoint)) : fail('invalid_period')
  },
  /** `emailEnabled` dirakit di `browser.ts` dari `browserEmail`. */
  inboxSummary: async (): Promise<Result<InboxSummaryDto>> =>
    ok({ pending: capturesWith('pending').length, unparsed: state.unparsed.length, emailEnabled: false }),
  listPendingCaptures: async (): Promise<Result<CaptureDto[]>> =>
    ok(capturesWith('pending').sort(byCaptureNewest).map((c) => {
      const dto = captureDraft(c)
      return { ...dto, duplicates: duplicatesOf({ ...c, ...dto }) }
    })),
  listUnparsedEmails: async (): Promise<Result<UnparsedEmailDto[]>> => ok(state.unparsed.map((u) => ({ ...u }))),
  confirmCapture: async (i: string, form: TxnFormDto): Promise<Result<SavedTxnDto>> => {
    const c = pendingCapture(i)
    if (c.status === 'error') return c
    const periods = periodsOf([form.date])
    const before = budgetLevels(periods)
    const r = writeTxn(null, form)
    if (r.status === 'error') return r
    Object.assign(c.data, { status: 'confirmed', txnId: r.data })
    return ok({ id: r.data, alerts: budgetAlerts(periods, before) })
  },
  linkCapture: async (i: string, txnId: string): Promise<Result<null>> => {
    const c = pendingCapture(i)
    if (c.status === 'error') return c
    if (!activeTxns().some((t) => t.id === txnId)) return notFound('transaction')
    Object.assign(c.data, { status: 'confirmed', txnId })
    return ok(null)
  },
  ignoreCapture: async (i: string): Promise<Result<null>> => {
    const c = pendingCapture(i)
    if (c.status === 'error') return c
    Object.assign(c.data, { status: 'ignored', txnId: null })
    return ok(null)
  },
  ignoreCapturesUntil: async (until: string): Promise<Result<string[]>> => {
    const hit = capturesWith('pending')
      .filter((c) => c.date <= until)
      .sort((a, b) => a.date.localeCompare(b.date) || a.id.localeCompare(b.id))
    for (const c of hit) Object.assign(c, { status: 'ignored', txnId: null })
    return ok(hit.map((c) => c.id))
  },
  /** Semua harus `ignored`; bila tidak, tidak ada yang berubah (K8). */
  unignoreCaptures: async (ids: string[]): Promise<Result<null>> => {
    const hit: Capture[] = []
    for (const i of ids) {
      const c = state.captures.find((x) => x.id === i)
      if (!c) return notFound('email capture')
      if (c.status !== 'ignored') return fail('capture_not_ignored')
      hit.push(c)
    }
    for (const c of hit) Object.assign(c, { status: 'pending', txnId: null })
    return ok(null)
  },
  listEmailRules: async (): Promise<Result<EmailRuleDto[]>> =>
    ok(state.emailRules.filter((r) => !r.deleted).sort(byRuleOrder).map(({ deleted: _, ...r }) => ({
      ...r, pending: capturesWith('pending').filter((c) => c.ruleId === r.id).length,
    }))),
  createEmailRule: async (input: EmailRuleInputDto): Promise<Result<RuleSavedDto>> => {
    const r = normalizedRule(input)
    const checked = checkEmailRule(r)
    if (checked.status === 'error') return checked
    const rule = { ...r, id: id(), deleted: false }
    state.emailRules.push(rule)
    return ok({ id: rule.id, reparsed: reparse() })
  },
  updateEmailRule: async (i: string, input: EmailRuleInputDto): Promise<Result<RuleSavedDto>> => {
    const r = normalizedRule(input)
    const checked = checkEmailRule(r)
    if (checked.status === 'error') return checked
    const rule = findEmailRule(i, false)
    if (rule.status === 'error') return rule
    Object.assign(rule.data, r)
    return ok({ id: i, reparsed: reparse() })
  },
  /** Menonaktifkan tidak me-re-parse; mengaktifkan me-re-parse (K7). */
  setEmailRuleEnabled: async (i: string, enabled: boolean): Promise<Result<RuleSavedDto>> => {
    const rule = findEmailRule(i, false)
    if (rule.status === 'error') return rule
    rule.data.enabled = enabled
    return ok({ id: i, reparsed: enabled ? reparse() : { emails: 0, items: 0 } })
  },
  deleteEmailRule: async (i: string): Promise<Result<null>> => {
    const rule = findEmailRule(i, false)
    if (rule.status === 'error') return rule
    rule.data.deleted = true
    return ok(null)
  },
  restoreEmailRule: async (i: string): Promise<Result<RuleSavedDto>> => {
    const rule = findEmailRule(i, true)
    if (rule.status === 'error') return rule
    rule.data.deleted = false
    return ok({ id: i, reparsed: reparse() })
  },
  /** Uji tanpa menulis apa pun; email tempelan dianggap baru diterima (K5, K13). */
  testEmailRule: async (input: EmailRuleInputDto, ruleId: string | null, sample: RuleSampleDto): Promise<Result<RuleTestDto>> => {
    const r = normalizedRule(input)
    const field = validate(r)
    if (field !== null) return fail('email_rule_invalid', field)
    let msg: Mail
    if (sample.kind === 'unparsed') {
      const m = unparsedMessage(sample.messageId)
      if (m.status === 'error') return m
      msg = m.data
    } else {
      msg = pasted(sample.sender, sample.subject, sample.text, Math.floor(Date.now() / 1000))
    }
    const t = trace(r, msg, localOffset())
    if (typeof t === 'string') return fail('email_rule_invalid', t)
    // Sama dengan re-parse saat disimpan: rule aktif lain (urutan prioritas) bisa lebih dulu menangkap
    // atau menahan email. Rule ini dihitung seolah aktif.
    const rules: (RulePatterns & Pick<EmailRuleInputDto, 'name' | 'priority'>)[] = activeEmailRules().filter((x) => x.id !== ruleId)
    let at = rules.findIndex((x) => x.priority < r.priority || (x.priority === r.priority && r.name < x.name))
    if (at < 0) at = rules.length
    rules.splice(at, 0, r)
    const unparsedMatches = state.unparsed.filter((u) => {
      const m = storedMessage(u)
      return m !== null && apply(rules, m, localOffset())?.rule === at
    }).length
    return ok({ ...t, unparsedMatches })
  },
  suggestEmailRule: async (messageId: string): Promise<Result<EmailRuleInputDto>> => {
    const m = unparsedMessage(messageId)
    return m.status === 'error' ? m : ok(suggest(m.data))
  },
  reparseUnparsed: async (): Promise<Result<ReparseSummaryDto>> => ok(reparse()),
}

// --- Keamanan (tanpa kripto): password & recovery key disimpan di memori. ---
// Urutan pemeriksaan mengikuti `crates/app/src/security.rs` dan `vault.rs`.

const CROCKFORD = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'
const RECOVERY_CHARS = 52

const security = {
  password: null as string | null,
  /** Recovery key ternormalisasi (52 karakter). */
  recovery: null as string | null,
  locked: false,
}

/** Panjang dalam code point, seperti `chars().count()` di Rust. */
const passwordOk = (pw: string) => [...pw].length >= 8

function crockford(bytes: Uint8Array): string {
  let out = ''
  let buf = 0
  let bits = 0
  for (const b of bytes) {
    buf = (buf << 8) | b
    bits += 8
    while (bits >= 5) {
      out += CROCKFORD[(buf >> (bits - 5)) & 31]
      bits -= 5
    }
  }
  if (bits > 0) out += CROCKFORD[(buf << (5 - bits)) & 31]
  return out
}

/** Seperti `parse_recovery_key`: buang spasi/`-`, uppercase, O→0, I/L→1. */
function normalizeRecovery(s: string): string | null {
  const n = s.replace(/[\s-]/g, '').toUpperCase().replace(/O/g, '0').replace(/[IL]/g, '1')
  return n.length === RECOVERY_CHARS && [...n].every((c) => CROCKFORD.includes(c)) ? n : null
}

const formatRecovery = (n: string) => n.match(/.{1,4}/g)!.join('-')

/** Semua command lain menolak dengan `locked` selama terkunci. */
export const isBrowserLocked = () => security.locked

export const browserSecurity = {
  securityStatus: async (): Promise<Result<SecurityStatusDto>> => {
    const encrypted = security.password !== null
    return ok({ encrypted, locked: security.locked, hasKeyfile: encrypted })
  },
  unlock: async (password: string): Promise<Result<null>> => {
    if (!security.locked) return ok(null)
    if (password !== security.password) return fail('wrong_password')
    security.locked = false
    return ok(null)
  },
  unlockWithRecovery: async (recoveryKey: string, newPassword: string): Promise<Result<null>> => {
    if (!security.locked) return ok(null)
    if (!passwordOk(newPassword)) return fail('password_too_short')
    const n = normalizeRecovery(recoveryKey)
    if (n === null || n !== security.recovery) return fail('invalid_recovery_key')
    security.password = newPassword
    security.locked = false
    return ok(null)
  },
  lock: async (): Promise<Result<null>> => {
    if (security.locked) return ok(null)
    if (security.password === null) return fail('encryption_state')
    security.locked = true
    return ok(null)
  },
  enableEncryption: async (password: string): Promise<Result<string>> => {
    if (security.password !== null) return fail('encryption_state')
    if (!passwordOk(password)) return fail('password_too_short')
    const n = crockford(crypto.getRandomValues(new Uint8Array(32)))
    security.password = password
    security.recovery = n
    return ok(formatRecovery(n))
  },
  disableEncryption: async (password: string): Promise<Result<null>> => {
    if (security.password === null) return fail('encryption_state')
    if (password !== security.password) return fail('wrong_password')
    security.password = null
    security.recovery = null
    return ok(null)
  },
  changePassword: async (oldPassword: string, newPassword: string): Promise<Result<null>> => {
    if (security.password === null) return fail('encryption_state')
    if (!passwordOk(newPassword)) return fail('password_too_short')
    if (oldPassword !== security.password) return fail('wrong_password')
    security.password = newPassword
    return ok(null)
  },
}

// --- Pemulihan (P11): aktif hanya lewat hook test; tidak ada file sungguhan di browser. ---

const recovery = { reason: null as RecoveryReasonDto | null }

/** Seperti `AppState.recovery`: command ledger menolak dengan `recovery_required`. */
export const isBrowserRecovering = () => recovery.reason !== null

/** Hook test/e2e: simulasikan DB yang gagal dibuka saat startup. */
export function __enterRecovery(reason: RecoveryReasonDto): void {
  recovery.reason = reason
}

export function __leaveRecovery(): void {
  recovery.reason = null
}

/** Seperti `recovery_slot`: hanya saat DB tidak terbuka (pemulihan atau terkunci). */
const recoveryAllowed = () => recovery.reason !== null || security.locked

function findBackup(source: BackupSourceDto): Snapshot | undefined {
  return source.kind === 'named' ? backups.find((b) => b.info.fileName === source.fileName) : undefined
}

/** DB pengganti plaintext: key file lama dipindah ke samping, seperti `restore_into_fresh`. */
function leaveRecoveryPlain() {
  security.password = null
  security.recovery = null
  security.locked = false
  recovery.reason = null
}

export const browserRecovery = {
  recoveryStatus: async (): Promise<RecoveryStatusDto> => ({
    active: recovery.reason !== null,
    reason: recovery.reason,
    dataDir: '',
    hasKeyfile: security.password !== null,
  }),
  // Snapshot browser tidak terenkripsi.
  inspectBackupSource: async (source: BackupSourceDto): Promise<Result<BackupSourceInfoDto>> =>
    findBackup(source) ? ok({ needsKey: false }) : fail('backup_invalid'),
  recoverFromBackup: async (source: BackupSourceDto, _secret: BackupSecretDto): Promise<Result<null>> => {
    if (!recoveryAllowed()) return fail('recovery_unavailable')
    const src = findBackup(source)
    if (!src) return fail('backup_invalid')
    Object.assign(state, structuredClone(src.state))
    leaveRecoveryPlain()
    return ok(null)
  },
  recoveryStartFresh: async (): Promise<Result<null>> => {
    if (!recoveryAllowed()) return fail('recovery_unavailable')
    Object.assign(state, structuredClone(FRESH_STATE))
    leaveRecoveryPlain()
    return ok(null)
  },
}
