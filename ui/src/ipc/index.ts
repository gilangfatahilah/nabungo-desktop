// Facade IPC: satu-satunya pintu UI ke backend. Komponen tidak memakai `commands` langsung.
import { isTauri } from '@tauri-apps/api/core'
import {
  commands,
  events,
  type CaptureDto,
  type EmailRuleDto,
  type EmailRuleInputDto,
  type ReparseSummaryDto,
  type RuleSampleDto,
  type RuleSavedDto,
  type RuleTestDto,
  type EmailStatusDto,
  type InboxSummaryDto,
  type SyncSummaryDto,
  type UnparsedEmailDto,
  type RecurrenceDto,
  type RuleDto,
  type RuleInputDto,
  type AccountFieldsDto,
  type AccountKindDto,
  type AccountsOverviewDto,
  type AllocationOverviewDto,
  type AllocationPointDto,
  type AppInfo,
  type AttachmentDto,
  type PayeeUsageDto,
  type TagUsageDto,
  type BackupDto,
  type BackupSecretDto,
  type BackupSourceDto,
  type BackupSourceInfoDto,
  type RecoveryStatusDto,
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
import { browserEmail, type EmailSyncHandlers } from './browser-email'

export type {
  CaptureDto,
  EmailRuleDto,
  EmailRuleInputDto,
  EmailRuleKindDto,
  ReparseSummaryDto,
  RuleSampleDto,
  RuleSavedDto,
  RuleTestDto,
  RuleTestItemDto,
  TraceFailDto,
  DuplicateTxnDto,
  InboxSummaryDto,
  UnparsedEmailDto,
  AccountDto,
  AccountFieldsDto,
  AccountGroupDto,
  AccountKindDto,
  AccountsOverviewDto,
  AllocationOverviewDto,
  AllocationPointDto,
  AppInfo,
  AttachmentDto,
  BackupDto,
  BackupKindDto,
  BackupSecretDto,
  BackupSourceDto,
  BackupSourceInfoDto,
  RecoveryReasonDto,
  RecoveryStatusDto,
  BucketActualDto,
  BucketBpDto,
  BucketDto,
  BucketFieldsDto,
  BudgetAlertDto,
  BudgetLevelDto,
  BudgetOverviewDto,
  CashflowPointDto,
  CategoryAmountDto,
  EmailStatusDto,
  EmailSyncDone,
  EmailSyncProgress,
  MappingDto,
  SavedTxnDto,
  NetWorthPointDto,
  ShareDto,
  UnallocatedDto,
  ValuationPointDto,
  ContactDto,
  ReconciliationDto,
  FreqDto,
  RecurrenceDto,
  RuleDto,
  RuleInputDto,
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
  PayeeUsageDto,
  PostingDto,
  SecurityStatusDto,
  SplitDto,
  SyncSummaryDto,
  StartupReportDto,
  TagUsageDto,
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
  TxnSortDto,
  TxnSortFieldDto,
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
let recoveryHandler: (() => void) | null = null

/** Dipanggil setiap kali backend menjawab `locked` (DB terenkripsi belum dibuka). */
export function setLockedHandler(fn: (() => void) | null): void {
  lockedHandler = fn
}

/** Dipanggil setiap kali backend menjawab `recovery_required` (DB gagal dibuka saat startup). */
export function setRecoveryHandler(fn: (() => void) | null): void {
  recoveryHandler = fn
}

async function unwrap<T>(p: Promise<Result<T>>): Promise<T> {
  const r = await p
  if (r.status === 'error') {
    if (r.error.code === 'locked') lockedHandler?.()
    if (r.error.code === 'recovery_required') recoveryHandler?.()
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

export async function setFontScale(scale: number): Promise<void> {
  await unwrap(api().setFontScale(scale))
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

export function listRules(): Promise<RuleDto[]> {
  return unwrap(api().listRules())
}

export function createRule(input: RuleInputDto): Promise<string> {
  return unwrap(api().createRule(input))
}

export async function updateRule(id: string, input: RuleInputDto): Promise<void> {
  await unwrap(api().updateRule(id, input))
}

export async function deleteRule(id: string): Promise<void> {
  await unwrap(api().deleteRule(id))
}

/** `form = null` → template apa adanya pada `date`. */
export function confirmOccurrence(id: string, date: string, form: TxnFormDto | null): Promise<SavedTxnDto> {
  return unwrap(api().confirmOccurrence(id, date, form))
}

export async function skipOccurrence(id: string, date: string): Promise<void> {
  await unwrap(api().skipOccurrence(id, date))
}

/** Mengembalikan jumlah transaksi yang dicatat. */
export function confirmAllDue(id: string): Promise<number> {
  return unwrap(api().confirmAllDue(id))
}

export function previewRecurrence(recurrence: RecurrenceDto, start: string, count: number): Promise<string[]> {
  return unwrap(api().previewRecurrence(recurrence, start, count))
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

// --- Tag & payee (CAT-04) ---

export function listTagUsage(): Promise<TagUsageDto[]> {
  return unwrap(api().listTagUsage())
}

export function createTag(name: string): Promise<string> {
  return unwrap(api().createTag(name))
}

export async function renameTag(id: string, name: string): Promise<void> {
  await unwrap(api().renameTag(id, name))
}

export async function mergeTag(sourceId: string, targetId: string): Promise<void> {
  await unwrap(api().mergeTag(sourceId, targetId))
}

export async function deleteTag(id: string): Promise<void> {
  await unwrap(api().deleteTag(id))
}

export function listPayeeUsage(): Promise<PayeeUsageDto[]> {
  return unwrap(api().listPayeeUsage())
}

export function createPayee(name: string, defaultCategoryId: string | null): Promise<string> {
  return unwrap(api().createPayee(name, defaultCategoryId))
}

export async function updatePayee(id: string, name: string, defaultCategoryId: string | null): Promise<void> {
  await unwrap(api().updatePayee(id, name, defaultCategoryId))
}

export async function mergePayee(sourceId: string, targetId: string): Promise<void> {
  await unwrap(api().mergePayee(sourceId, targetId))
}

export async function deletePayee(id: string): Promise<void> {
  await unwrap(api().deletePayee(id))
}

// --- Lampiran (TRX-08) ---

/** Membuka dialog pilih file; batal → `[]`. */
export function addAttachments(txnId: string): Promise<AttachmentDto[]> {
  return unwrap(api().addAttachments(txnId))
}

export function listAttachments(txnId: string): Promise<AttachmentDto[]> {
  return unwrap(api().listAttachments(txnId))
}

export async function removeAttachment(id: string): Promise<void> {
  await unwrap(api().removeAttachment(id))
}

export async function openAttachment(id: string): Promise<void> {
  await unwrap(api().openAttachment(id))
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

export function accountBalanceAsOf(accountId: string, date: string): Promise<number> {
  return unwrap(api().accountBalanceAsOf(accountId, date))
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

/** Dasbor pertama tampil: backend mencatat `startup_ms` ke log lokal (sekali per proses). */
export function appReady(): Promise<void> {
  return api().appReady()
}

/** Backup otomatis/pra-migrasi/pra-restore, terbaru dulu. */
export function listBackups(): Promise<BackupDto[]> {
  return unwrap(api().listBackups())
}

/** Dialog simpan; `null` bila dibatalkan, path hasil bila sukses. */
export function backupNow(): Promise<string | null> {
  return unwrap(api().backupNow())
}

/** Setelah sukses, UI wajib memuat ulang semua data. `srcRecoveryKey`: recovery key milik
 *  backup terenkripsi dari instalasi lain. */
export async function restoreBackup(source: BackupSourceDto, srcRecoveryKey: string | null = null): Promise<void> {
  await unwrap(api().restoreBackup(source, srcRecoveryKey))
}

/** Dialog buka file backup; `null` bila dibatalkan. */
export function pickBackupFile(): Promise<string | null> {
  return unwrap(api().pickBackupFile())
}

/** `needsKey`: backup terenkripsi dengan key selain key DB aktif (mis. instalasi lain). */
export function inspectBackupSource(source: BackupSourceDto): Promise<BackupSourceInfoDto> {
  return unwrap(api().inspectBackupSource(source))
}

export function recoveryStatus(): Promise<RecoveryStatusDto> {
  return api().recoveryStatus()
}

/** Ganti DB yang tidak bisa dibuka dengan backup; DB lama dipindah ke samping, tidak dihapus. */
export async function recoverFromBackup(source: BackupSourceDto, secret: BackupSecretDto): Promise<void> {
  await unwrap(api().recoverFromBackup(source, secret))
}

/** Mulai dengan DB kosong (onboarding); DB lama dipindah ke samping. */
export async function recoveryStartFresh(): Promise<void> {
  await unwrap(api().recoveryStartFresh())
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

// --- Email capture (EML-01) ---

export function emailStatus(): Promise<EmailStatusDto> {
  return unwrap(api().emailStatus())
}

/** `password: null` (atau kosong) = pakai App Password tersimpan. */
export async function connectEmail(address: string, mailbox: string, password: string | null): Promise<void> {
  await unwrap(api().connectEmail(address, mailbox, password))
}

export async function disconnectEmail(): Promise<void> {
  await unwrap(api().disconnectEmail())
}

export function syncEmail(): Promise<SyncSummaryDto> {
  return unwrap(api().syncEmail())
}

/** Kemajuan & akhir sync (manual maupun terjadwal). Mengembalikan fungsi berhenti mendengar. */
export async function onEmailSync(h: EmailSyncHandlers): Promise<() => void> {
  if (!isTauri()) return browserEmail.subscribe(h)
  const offs = await Promise.all([
    events.emailSyncProgress.listen((e) => h.progress?.(e.payload)),
    events.emailSyncDone.listen((e) => h.done?.(e.payload)),
  ])
  return () => offs.forEach((off) => off())
}

// --- Inbox email capture (EML-03) ---

export function inboxSummary(): Promise<InboxSummaryDto> {
  return unwrap(api().inboxSummary())
}

export function listPendingCaptures(): Promise<CaptureDto[]> {
  return unwrap(api().listPendingCaptures())
}

export function listUnparsedEmails(): Promise<UnparsedEmailDto[]> {
  return unwrap(api().listUnparsedEmails())
}

export function confirmCapture(id: string, form: TxnFormDto): Promise<SavedTxnDto> {
  return unwrap(api().confirmCapture(id, form))
}

/** Hubungkan ke transaksi yang sudah ada (duplikat) tanpa membuat transaksi baru. */
export async function linkCapture(id: string, txnId: string): Promise<void> {
  await unwrap(api().linkCapture(id, txnId))
}

export async function ignoreCapture(id: string): Promise<void> {
  await unwrap(api().ignoreCapture(id))
}

/** Abaikan semua pending bertanggal ≤ `date`; id yang diabaikan dikembalikan untuk undo. */
export function ignoreCapturesUntil(date: string): Promise<string[]> {
  return unwrap(api().ignoreCapturesUntil(date))
}

export async function unignoreCaptures(ids: string[]): Promise<void> {
  await unwrap(api().unignoreCaptures(ids))
}

// --- Rule email capture (EML-04) ---

export function listEmailRules(): Promise<EmailRuleDto[]> {
  return unwrap(api().listEmailRules())
}

/** Simpan lalu re-parse email gagal dibaca; `reparsed` = yang pindah ke pending. */
export function createEmailRule(input: EmailRuleInputDto): Promise<RuleSavedDto> {
  return unwrap(api().createEmailRule(input))
}

export function updateEmailRule(id: string, input: EmailRuleInputDto): Promise<RuleSavedDto> {
  return unwrap(api().updateEmailRule(id, input))
}

export function setEmailRuleEnabled(id: string, enabled: boolean): Promise<RuleSavedDto> {
  return unwrap(api().setEmailRuleEnabled(id, enabled))
}

export async function deleteEmailRule(id: string): Promise<void> {
  await unwrap(api().deleteEmailRule(id))
}

export function restoreEmailRule(id: string): Promise<RuleSavedDto> {
  return unwrap(api().restoreEmailRule(id))
}

/** Uji rule tanpa menyimpan apa pun. */
/** `ruleId` = rule yang sedang diubah: versi tersimpannya tidak ikut dihitung di `unparsedMatches`. */
export function testEmailRule(input: EmailRuleInputDto, ruleId: string | null, sample: RuleSampleDto): Promise<RuleTestDto> {
  return unwrap(api().testEmailRule(input, ruleId, sample))
}

export function suggestEmailRule(messageId: string): Promise<EmailRuleInputDto> {
  return unwrap(api().suggestEmailRule(messageId))
}

export function reparseUnparsed(): Promise<ReparseSummaryDto> {
  return unwrap(api().reparseUnparsed())
}
