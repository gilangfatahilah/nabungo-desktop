// Facade IPC: satu-satunya pintu UI ke backend. Komponen tidak memakai `commands` langsung.
import { isTauri } from '@tauri-apps/api/core'
import {
  commands,
  type AccountFieldsDto,
  type AccountKindDto,
  type AccountsOverviewDto,
  type AllocationOverviewDto,
  type AllocationPointDto,
  type AppInfo,
  type BackupDto,
  type BucketFieldsDto,
  type BudgetAlertDto,
  type BudgetOverviewDto,
  type CashflowPointDto,
  type CategoryAmountDto,
  type CategoryDto,
  type CategoryFieldsDto,
  type CategoryKindDto,
  type IpcError,
  type LocaleDto,
  type NetWorthPointDto,
  type OnboardingDto,
  type ThemeDto,
  type TxnDetailDto,
  type TxnFormDefaultsDto,
  type TxnFormDto,
  type TxnPageDto,
  type TxnQueryDto,
  type PayeeSuggestionDto,
  type SecurityStatusDto,
  type ShareDto,
  type SavedTxnDto,
  type StartupReportDto,
  type UiSettingsDto,
  type ValuationPointDto,
  type ContactDto,
  type ReconciliationDto,
} from './bindings'
import { browserCommands } from './browser'

export type {
  AccountDto,
  AccountFieldsDto,
  AccountGroupDto,
  AccountKindDto,
  AccountsOverviewDto,
  AllocationOverviewDto,
  AllocationPointDto,
  AppInfo,
  BackupDto,
  BackupKindDto,
  BucketActualDto,
  BucketBpDto,
  BucketDto,
  BucketFieldsDto,
  BudgetAlertDto,
  BudgetLevelDto,
  BudgetOverviewDto,
  CashflowPointDto,
  CategoryAmountDto,
  MappingDto,
  SavedTxnDto,
  NetWorthPointDto,
  ShareDto,
  UnallocatedDto,
  ValuationPointDto,
  ContactDto,
  ReconciliationDto,
  WarningDto,
  BudgetRowDto,
  BudgetTotalDto,
  CategoryDto,
  CategoryFieldsDto,
  CategoryKindDto,
  ErrorCode,
  IntegrityStatusDto,
  IpcError,
  LocaleDto,
  NetWorthDto,
  OnboardingDto,
  PayeeSuggestionDto,
  PostingDto,
  SecurityStatusDto,
  SplitDto,
  StartupReportDto,
  FeeDto,
  ThemeDto,
  TxnDetailDto,
  TxnFormDefaultsDto,
  TxnFormDto,
  TxnFormKindDto,
  TxnKindDto,
  TxnPageDto,
  TxnQueryDto,
  TxnRowDto,
  UiSettingsDto,
} from './bindings'

/** Error domain dari backend; `error.code` diterjemahkan oleh UI. */
export class IpcFailure extends Error {
  readonly error: IpcError

  constructor(error: IpcError) {
    super(error.entity ? `${error.code}: ${error.entity}` : error.code)
    this.name = 'IpcFailure'
    this.error = error
  }
}

type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: IpcError }

function api(): typeof browserCommands {
  return isTauri() ? commands : browserCommands
}

let lockedHandler: (() => void) | null = null

/** Dipanggil setiap kali backend menjawab `locked` (DB terenkripsi belum dibuka). */
export function setLockedHandler(fn: (() => void) | null): void {
  lockedHandler = fn
}

async function unwrap<T>(p: Promise<Result<T>>): Promise<T> {
  const r = await p
  if (r.status === 'error') {
    if (r.error.code === 'locked') lockedHandler?.()
    throw new IpcFailure(r.error)
  }
  return r.data
}

export function appInfo(): Promise<AppInfo> {
  return api().appInfo()
}

export function getUiSettings(): Promise<UiSettingsDto> {
  return unwrap(api().getUiSettings())
}

export async function setTheme(theme: ThemeDto): Promise<void> {
  await unwrap(api().setTheme(theme))
}

export async function setLocale(locale: LocaleDto): Promise<void> {
  await unwrap(api().setLocale(locale))
}

export function parseAmount(input: string): Promise<number> {
  return unwrap(api().parseAmount(input))
}

// --- Onboarding & periode budget ---

export function getOnboardingStatus(): Promise<boolean> {
  return unwrap(api().getOnboardingStatus())
}

/** Mengembalikan ID akun pertama. */
export function completeOnboarding(input: OnboardingDto): Promise<string> {
  return unwrap(api().completeOnboarding(input))
}

export function getPeriodStartDay(): Promise<number> {
  return unwrap(api().getPeriodStartDay())
}

export async function setPeriodStartDay(day: number): Promise<void> {
  await unwrap(api().setPeriodStartDay(day))
}

// --- Akun & account group ---

export function accountsOverview(): Promise<AccountsOverviewDto> {
  return unwrap(api().accountsOverview())
}

export function createAccount(kind: AccountKindDto, fields: AccountFieldsDto): Promise<string> {
  return unwrap(api().createAccount(kind, fields))
}

export async function updateAccount(id: string, fields: AccountFieldsDto): Promise<void> {
  await unwrap(api().updateAccount(id, fields))
}

export async function archiveAccount(id: string): Promise<void> {
  await unwrap(api().archiveAccount(id))
}

export async function unarchiveAccount(id: string): Promise<void> {
  await unwrap(api().unarchiveAccount(id))
}

export async function deleteAccount(id: string): Promise<void> {
  await unwrap(api().deleteAccount(id))
}

export async function reorderAccounts(groupId: string | null, ids: string[]): Promise<void> {
  await unwrap(api().reorderAccounts(groupId, ids))
}

export function createAccountGroup(name: string): Promise<string> {
  return unwrap(api().createAccountGroup(name))
}

export async function renameAccountGroup(id: string, name: string): Promise<void> {
  await unwrap(api().renameAccountGroup(id, name))
}

export async function deleteAccountGroup(id: string): Promise<void> {
  await unwrap(api().deleteAccountGroup(id))
}

export async function reorderAccountGroups(ids: string[]): Promise<void> {
  await unwrap(api().reorderAccountGroups(ids))
}

// --- Kategori ---

export function listCategories(): Promise<CategoryDto[]> {
  return unwrap(api().listCategories())
}

export function createCategory(kind: CategoryKindDto, fields: CategoryFieldsDto): Promise<string> {
  return unwrap(api().createCategory(kind, fields))
}

export async function updateCategory(id: string, fields: CategoryFieldsDto): Promise<void> {
  await unwrap(api().updateCategory(id, fields))
}

export async function archiveCategory(id: string): Promise<void> {
  await unwrap(api().archiveCategory(id))
}

export async function unarchiveCategory(id: string): Promise<void> {
  await unwrap(api().unarchiveCategory(id))
}

export async function deleteCategory(id: string): Promise<void> {
  await unwrap(api().deleteCategory(id))
}

export async function mergeCategory(source: string, target: string): Promise<void> {
  await unwrap(api().mergeCategory(source, target))
}

// --- Transaksi ---

/** Mengembalikan id transaksi baru dan peringatan budget yang baru terlewati. */
export function createTxn(form: TxnFormDto): Promise<SavedTxnDto> {
  return unwrap(api().createTxn(form))
}

/** Mengembalikan peringatan budget yang baru terlewati. */
export function updateTxn(id: string, form: TxnFormDto): Promise<BudgetAlertDto[]> {
  return unwrap(api().updateTxn(id, form))
}

export function getTxn(id: string): Promise<TxnDetailDto> {
  return unwrap(api().getTxn(id))
}

export function listTxns(query: TxnQueryDto): Promise<TxnPageDto> {
  return unwrap(api().listTxns(query))
}

export async function deleteTxn(id: string): Promise<void> {
  await unwrap(api().deleteTxn(id))
}

export async function undoDeleteTxn(id: string): Promise<void> {
  await unwrap(api().undoDeleteTxn(id))
}

export function payeeSuggestion(name: string): Promise<PayeeSuggestionDto | null> {
  return unwrap(api().payeeSuggestion(name))
}

export function searchPayees(query: string): Promise<string[]> {
  return unwrap(api().searchPayees(query))
}

export function listTags(): Promise<string[]> {
  return unwrap(api().listTags())
}

export function txnFormDefaults(): Promise<TxnFormDefaultsDto> {
  return unwrap(api().txnFormDefaults())
}

// --- Budget ---

/** Kunci periode budget (`YYYY-MM`) yang memuat tanggal ISO `date`. */
export function periodOf(date: string): Promise<string> {
  return unwrap(api().periodOf(date))
}

export function budgetOverview(period: string): Promise<BudgetOverviewDto> {
  return unwrap(api().budgetOverview(period))
}

/** `amount = null` menghapus budget. */
export async function setBudget(categoryId: string, period: string, amount: number | null): Promise<void> {
  await unwrap(api().setBudget(categoryId, period, amount))
}

/** Budget kategori × periode harus sudah ada. */
export async function setBudgetRollover(categoryId: string, period: string, on: boolean): Promise<void> {
  await unwrap(api().setBudgetRollover(categoryId, period, on))
}

/** Mengembalikan jumlah budget yang disalin dari periode sebelumnya. */
export function copyBudgets(period: string): Promise<number> {
  return unwrap(api().copyBudgets(period))
}

// --- Valuation ---

export function recordValuation(accountId: string, date: string, value: number): Promise<string> {
  return unwrap(api().recordValuation(accountId, date, value))
}

export function valuationHistory(accountId: string): Promise<ValuationPointDto[]> {
  return unwrap(api().valuationHistory(accountId))
}

// --- Rekonsiliasi & kontak ---

export function reconcileAccount(accountId: string, date: string, balance: number): Promise<ReconciliationDto> {
  return unwrap(api().reconcileAccount(accountId, date, balance))
}

export function listContacts(): Promise<ContactDto[]> {
  return unwrap(api().listContacts())
}

// --- Alokasi modal ---

export function allocationOverview(): Promise<AllocationOverviewDto> {
  return unwrap(api().listAllocation())
}

export function createBucket(fields: BucketFieldsDto): Promise<string> {
  return unwrap(api().createBucket(fields))
}

export async function updateBucket(id: string, fields: BucketFieldsDto): Promise<void> {
  await unwrap(api().updateBucket(id, fields))
}

export async function deleteBucket(id: string): Promise<void> {
  await unwrap(api().deleteBucket(id))
}

/** Mengganti seluruh porsi akun; `[]` = akun tidak dialokasikan. */
export async function setAccountAllocation(accountId: string, shares: ShareDto[]): Promise<void> {
  await unwrap(api().setAccountAllocation(accountId, shares))
}

/** `count` periode berakhir di `endPeriod`, terlama dulu. */
export function allocationHistory(endPeriod: string, count: number): Promise<AllocationPointDto[]> {
  return unwrap(api().allocationHistory(endPeriod, count))
}

// --- Laporan ---

/** `count` periode (maks. 24) berakhir di `endPeriod`, terlama dulu. */
export function cashflowReport(endPeriod: string, count: number): Promise<CashflowPointDto[]> {
  return unwrap(api().cashflowReport(endPeriod, count))
}

/** Total per kategori root, nominal terbesar dulu; kategori bernilai 0 tidak muncul. */
export function categoryBreakdown(period: string, kind: CategoryKindDto): Promise<CategoryAmountDto[]> {
  return unwrap(api().categoryBreakdown(period, kind))
}

/** Net worth per akhir periode, `count` periode (maks. 24) berakhir di `endPeriod`. */
export function netWorthHistory(endPeriod: string, count: number): Promise<NetWorthPointDto[]> {
  return unwrap(api().netWorthHistory(endPeriod, count))
}

// --- Backup, pemulihan & ekspor ---

/** Status sesi sebelumnya (shutdown tidak bersih, hasil integrity check). */
export function startupReport(): Promise<StartupReportDto> {
  return api().startupReport()
}

/** Backup otomatis/pra-migrasi/pra-restore, terbaru dulu. */
export function listBackups(): Promise<BackupDto[]> {
  return unwrap(api().listBackups())
}

/** Dialog simpan; `null` bila dibatalkan, path hasil bila sukses. */
export function backupNow(): Promise<string | null> {
  return unwrap(api().backupNow())
}

/** Setelah sukses, UI wajib memuat ulang semua data. */
export async function restoreBackup(fileName: string): Promise<void> {
  await unwrap(api().restoreBackup(fileName))
}

/** Dialog buka file; `false` bila dibatalkan. */
export function restoreBackupFromFile(): Promise<boolean> {
  return unwrap(api().restoreBackupFromFile())
}

export function exportJson(): Promise<string | null> {
  return unwrap(api().exportJson())
}

/** Dialog pilih folder; mengembalikan subfolder ekspor yang dibuat. */
export function exportCsv(): Promise<string | null> {
  return unwrap(api().exportCsv())
}

// --- Keamanan (enkripsi DB) ---

/** Bisa dipanggil saat terkunci. */
export function securityStatus(): Promise<SecurityStatusDto> {
  return unwrap(api().securityStatus())
}

export async function unlock(password: string): Promise<void> {
  await unwrap(api().unlock(password))
}

/** Buka dengan recovery key lalu set password baru. */
export async function unlockWithRecovery(recoveryKey: string, newPassword: string): Promise<void> {
  await unwrap(api().unlockWithRecovery(recoveryKey, newPassword))
}

export async function lock(): Promise<void> {
  await unwrap(api().lock())
}

/** Mengembalikan recovery key; hanya ditampilkan sekali. */
export function enableEncryption(password: string): Promise<string> {
  return unwrap(api().enableEncryption(password))
}

export async function disableEncryption(password: string): Promise<void> {
  await unwrap(api().disableEncryption(password))
}

export async function changePassword(oldPassword: string, newPassword: string): Promise<void> {
  await unwrap(api().changePassword(oldPassword, newPassword))
}

export async function setAutoLockMinutes(minutes: number): Promise<void> {
  await unwrap(api().setAutoLockMinutes(minutes))
}
