// Ledger in-memory untuk mode browser (tanpa Tauri). Tanpa validasi domain: hanya cukup
// agar layar bisa dicoba. Aturan bisnis yang sebenarnya ada di Rust.
import type {
  AccountDto, AccountFieldsDto, AccountGroupDto, AccountKindDto, AccountsOverviewDto, CategoryDto,
  CategoryFieldsDto, CategoryKindDto, IpcError, OnboardingDto,
} from './bindings'

type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: IpcError }

const ok = <T>(data: T): Result<T> => ({ status: 'ok', data })
const notFound = (entity: string): Result<never> => ({ status: 'error', error: { code: 'not_found', entity } })

const LIABILITIES: AccountKindDto[] = ['credit_card', 'loan']

const state = {
  onboarded: false,
  periodStartDay: 1,
  groups: [] as AccountGroupDto[],
  accounts: [] as AccountDto[],
  categories: [] as CategoryDto[],
}

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
    // Mode browser tidak punya transaksi: saldo = saldo awal.
    balance: f.openingBalance,
    openingBalance: f.openingBalance,
    openingDate: f.openingBalance !== 0 ? f.openingDate : null,
  }
}

function overview(): AccountsOverviewDto {
  let assets = 0
  let liabilities = 0
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
    return ok(a.id)
  },
  updateAccount: async (i: string, fields: AccountFieldsDto): Promise<Result<null>> => {
    const n = state.accounts.findIndex((a) => a.id === i)
    const cur = state.accounts[n]
    if (!cur) return notFound('account')
    state.accounts[n] = toAccount(cur.kind, fields, cur)
    return ok(null)
  },
  archiveAccount: async (i: string) => setArchived(state.accounts, i, true, 'account'),
  unarchiveAccount: async (i: string) => setArchived(state.accounts, i, false, 'account'),
  deleteAccount: async (i: string) => {
    state.accounts = state.accounts.filter((a) => a.id !== i)
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
}
