// Ledger in-memory untuk mode browser (tanpa Tauri): cukup agar layar bisa dicoba.
// Aturan bisnis yang sebenarnya ada di Rust; aturan transaksi di sini (validasi form, saldo,
// saldo berjalan, filter) dicerminkan dari core/storage dan diuji di browser-ledger.test.ts.
import type {
  AccountDto, AccountFieldsDto, AccountGroupDto, AccountKindDto, AccountsOverviewDto, AllocationOverviewDto,
  AllocationPointDto, BucketActualDto, CashflowPointDto, CategoryAmountDto, NetWorthPointDto, BucketDto, BucketFieldsDto, BudgetOverviewDto, MappingDto, ShareDto,
  UnallocatedDto, WarningDto,
  BudgetRowDto, CategoryDto,
  CategoryFieldsDto, CategoryKindDto, ErrorCode, IpcError, OnboardingDto, PayeeSuggestionDto, PostingDto,
  TxnDetailDto, TxnFormDefaultsDto, TxnFormDto, TxnKindDto, TxnPageDto, TxnQueryDto, TxnRowDto,
  ValuationPointDto,
} from './bindings'

type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: IpcError }

const ok = <T>(data: T): Result<T> => ({ status: 'ok', data })
const fail = (code: ErrorCode, entity: string | null = null): Result<never> => ({ status: 'error', error: { code, entity } })
const notFound = (entity: string): Result<never> => fail('not_found', entity)

const LIABILITIES: AccountKindDto[] = ['credit_card', 'loan']

const state = {
  onboarded: false,
  periodStartDay: 1,
  groups: [] as AccountGroupDto[],
  accounts: [] as AccountDto[],
  categories: [] as CategoryDto[],
  txns: [] as Txn[],
  payees: [] as { id: string; name: string }[],
  lastFeeCategoryId: null as string | null,
  budgets: [] as { categoryId: string; period: string; amount: number }[],
  buckets: [] as BucketDto[],
  mappings: [] as MappingDto[],
}

type Txn = {
  id: string
  seq: number
  kind: TxnKindDto
  date: string
  payeeId: string | null
  note: string | null
  tags: string[]
  postings: PostingDto[]
  deleted: boolean
}

// Batas nominal per posting, sama dengan `MAX_AMOUNT` di core.
const MAX_AMOUNT = 1_000_000_000_000
const SEED_FEE_NAMES = ['biaya admin', 'fees']
let seq = 0

const id = () => crypto.randomUUID()

function nextSort(items: { sortOrder: number }[]): number {
  return items.reduce((m, x) => Math.max(m, x.sortOrder), 0) + 1
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

/** Seperti Rust: saldo awal ≠ 0 disimpan sebagai transaksi `opening`. */
function syncOpening(a: AccountDto) {
  state.txns = state.txns.filter((t) => !(t.kind === 'opening' && t.postings[0]?.accountId === a.id))
  if (a.openingBalance === 0 || !a.openingDate) return
  state.txns.push({
    id: id(), seq: ++seq, kind: 'opening', date: a.openingDate, payeeId: null, note: null, tags: [],
    postings: [{ accountId: a.id, categoryId: null, amount: a.openingBalance, memo: null }], deleted: false,
  })
}

function overview(): AccountsOverviewDto {
  let assets = 0
  let liabilities = 0
  for (const a of state.accounts) a.balance = balanceOf(a.id)
  for (const a of state.accounts) {
    // Seperti Rust: akun diarsip tetap dihitung; hanya `includeInNetWorth` yang menyaring.
    if (!a.includeInNetWorth) continue
    if (LIABILITIES.includes(a.kind)) liabilities += -a.balance
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
  const p = { id: id(), name: n }
  state.payees.push(p)
  return p.id
}

function normalizeTags(tags: string[]): string[] {
  const out: string[] = []
  for (const raw of tags) {
    const t = raw.trim().replace(/^#/, '').trim()
    if (t && !out.some((x) => x.toLowerCase() === t.toLowerCase())) out.push(t)
  }
  return out.sort((a, b) => a.localeCompare(b))
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
    Object.assign(existing, fields)
    return ok(existing.id)
  }
  const t: Txn = { id: id(), seq: ++seq, deleted: false, ...fields }
  state.txns.push(t)
  return ok(t.id)
}

const byNewest = (a: Txn, b: Txn) => b.date.localeCompare(a.date) || b.seq - a.seq
const payeeName = (t: Txn) => state.payees.find((p) => p.id === t.payeeId)?.name ?? null

function matches(t: Txn, q: TxnQueryDto): boolean {
  if (q.dateFrom && t.date < q.dateFrom) return false
  if (q.dateTo && t.date > q.dateTo) return false
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
    tags: [...t.tags], accountDelta: null, runningBalance: null,
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

function listPage(q: TxnQueryDto): TxnPageDto {
  const hits = activeTxns().filter((t) => matches(t, q)).sort(byNewest)
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

function budgetOverview(p: Period): BudgetOverviewDto {
  const [from, toEx] = browserPeriod.range(p, state.periodStartDay)
  const key = browserPeriod.key(p)
  const cats = state.categories.filter((c) => c.kind === 'expense')
  const direct = new Map<string, number>()
  for (const t of activeTxns()) {
    if (t.date < from || t.date >= toEx) continue
    for (const x of t.postings) if (x.categoryId) direct.set(x.categoryId, (direct.get(x.categoryId) ?? 0) + x.amount)
  }
  const spent = new Map<string, number>()
  const add = (k: string, v: number) => spent.set(k, (spent.get(k) ?? 0) + v)
  for (const c of cats) {
    const own = -(direct.get(c.id) ?? 0)
    add(c.id, own)
    if (c.parentId) add(c.parentId, own)
  }
  const budgetOf = (c: string) => state.budgets.find((b) => b.categoryId === c && b.period === key)?.amount ?? null
  const bySort = (a: CategoryDto, b: CategoryDto) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name)
  const rows: BudgetRowDto[] = []
  for (const root of cats.filter((c) => !c.parentId).sort(bySort)) {
    for (const c of [root, ...cats.filter((x) => x.parentId === root.id).sort(bySort)]) {
      const budget = budgetOf(c.id)
      const s = spent.get(c.id) ?? 0
      if (c.archived && budget === null && s === 0) continue
      rows.push({ categoryId: c.id, parentId: c.parentId, budget, spent: s, remaining: budget === null ? null : budget - s })
    }
  }
  // Total: baris berbudget yang parent-nya tidak berbudget (aturan K2).
  const top = rows.filter((r) => r.budget !== null && !(r.parentId && rows.some((x) => x.categoryId === r.parentId && x.budget !== null)))
  const total = { budget: top.reduce((m, r) => m + r.budget!, 0), spent: top.reduce((m, r) => m + r.spent, 0), remaining: 0 }
  total.remaining = total.budget - total.spent
  return {
    period: key, prev: browserPeriod.key(browserPeriod.shift(p, -1)), next: browserPeriod.key(browserPeriod.shift(p, 1)),
    from, to: previousDay(toEx), rows, total,
  }
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
  if (!checkBp(f.targetBp)) return fail('allocation_invalid_bp')
  const siblings = state.buckets.filter((b) => b.parentId === f.parentId && b.id !== selfId)
  if (siblings.reduce((m, b) => m + b.targetBp, f.targetBp) > FULL_BP) return fail('allocation_targets_exceed')
  if (siblings.some((b) => b.name.toLowerCase() === name.toLowerCase())) return fail('duplicate', 'bucket')
  return ok(name)
}

type Balance = { id: string; kind: AccountKindDto; balance: number; includeInNetWorth: boolean }

function allocationReport(accounts: Balance[]) {
  let gross = 0
  let liabilities = 0
  const counted = new Map<string, number>()
  for (const a of accounts.filter((x) => x.includeInNetWorth)) {
    if (LIABILITIES.includes(a.kind)) liabilities -= a.balance
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
  const actuals: BucketActualDto[] = buckets.map((b) => {
    const actual = (direct.get(b.id) ?? 0)
      + buckets.filter((c) => c.parentId === b.id).reduce((m, c) => m + (direct.get(c.id) ?? 0), 0)
    const parent = buckets.find((p) => p.id === b.parentId)
    const targetBp = parent ? mulDiv(parent.targetBp, b.targetBp, FULL_BP) : b.targetBp
    const actualBp = bpOf(actual)
    return {
      bucketId: b.id, targetBp, actual, actualBp, diffBp: actualBp - targetBp,
      diffAmount: actual - mulDiv(gross, targetBp, FULL_BP),
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
  for (const b of buckets) {
    if (warnings.some((w) => w.parentId === b.parentId)) continue
    const totalBp = buckets.filter((s) => s.parentId === b.parentId).reduce((m, s) => m + s.targetBp, 0)
    if (totalBp !== FULL_BP) warnings.push({ parentId: b.parentId, totalBp })
  }
  return { buckets, gross, liabilities, actuals, unallocated, unallocatedTotal, unallocatedBp: bpOf(unallocatedTotal), warnings }
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

function netWorthPoint(p: Period): NetWorthPointDto {
  const date = previousDay(browserPeriod.range(p, state.periodStartDay)[1])
  let assets = 0
  let liabilities = 0
  for (const b of balancesAsOf(date)) {
    if (!b.includeInNetWorth) continue
    if (LIABILITIES.includes(b.kind)) liabilities -= b.balance
    else assets += b.balance
  }
  return { period: browserPeriod.key(p), date, assets, liabilities, net: assets - liabilities }
}

export const browserLedger = {
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
  createTxn: async (form: TxnFormDto) => writeTxn(null, form),
  updateTxn: async (i: string, form: TxnFormDto): Promise<Result<null>> => {
    const t = activeTxns().find((x) => x.id === i)
    if (!t) return notFound('transaction')
    if (t.kind === 'opening') return fail('opening_not_allowed')
    if (t.kind === 'valuation' || t.kind === 'adjustment') return fail('txn_not_editable')
    const r = writeTxn(t, form)
    return r.status === 'ok' ? ok(null) : r
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
    const p = state.payees.find((x) => x.name.toLowerCase() === name.trim().toLowerCase())
    const last = p && activeTxns().filter((t) => t.payeeId === p.id).sort(byNewest)[0]
    if (!last) return ok(null)
    const row = shapeRow(last, null, null)
    return ok({ accountId: row.accountId, categoryId: row.categoryId })
  },
  searchPayees: async (query: string) => {
    const q = query.trim().toLowerCase()
    return ok(state.payees.filter((p) => p.name.toLowerCase().includes(q)).map((p) => p.name).slice(0, 8))
  },
  listTags: async () => ok(normalizeTags(activeTxns().flatMap((t) => t.tags))),
  txnFormDefaults: async (): Promise<Result<TxnFormDefaultsDto>> => {
    const last = activeTxns().filter((t) => t.kind !== 'opening').sort((a, b) => b.seq - a.seq)[0]
    return ok({ lastAccountId: last ? shapeRow(last, null, null).accountId : null, feeCategoryId: defaultFeeCategory() })
  },
  periodOf: async (date: string): Promise<Result<string>> =>
    ok(browserPeriod.key(browserPeriod.of(date, state.periodStartDay))),
  budgetOverview: async (period: string): Promise<Result<BudgetOverviewDto>> => {
    const p = browserPeriod.parse(period)
    return p ? ok(budgetOverview(p)) : fail('invalid_period')
  },
  setBudget: async (categoryId: string, period: string, amount: number | null): Promise<Result<null>> => {
    if (!browserPeriod.parse(period)) return fail('invalid_period')
    const c = state.categories.find((x) => x.id === categoryId)
    if (!c) return notFound('category')
    if (amount !== null && c.archived) return fail('archived', 'category')
    if (c.kind !== 'expense') return fail('budget_income_category')
    if (amount !== null && amount < 0) return fail('budget_negative')
    if (amount !== null && amount > MAX_AMOUNT) return fail('budget_too_large')
    state.budgets = state.budgets.filter((b) => !(b.categoryId === categoryId && b.period === period))
    if (amount !== null) state.budgets.push({ categoryId, period, amount })
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
      state.budgets.push({ categoryId: b.categoryId, period, amount: b.amount })
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
      id: id(), seq: ++seq, kind: 'valuation', date, payeeId: null, note: null, tags: [], deleted: false,
      postings: [{ accountId, categoryId: null, amount: change, memo: null }],
    }
    state.txns.push(t)
    return ok(t.id)
  },
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
      id: id(), parentId: fields.parentId, name: v.data, targetBp: fields.targetBp, color: fields.color,
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
    Object.assign(b, { name: v.data, parentId: fields.parentId, targetBp: fields.targetBp, color: fields.color, sortOrder })
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
    if (LIABILITIES.includes(a.kind)) return fail('allocation_liability')
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
  netWorthHistory: async (endPeriod: string, count: number): Promise<Result<NetWorthPointDto[]>> => {
    const end = browserPeriod.parse(endPeriod)
    return end ? ok(periodsEnding(end, count).map(netWorthPoint)) : fail('invalid_period')
  },
}
