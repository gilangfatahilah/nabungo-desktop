// State form transaksi di UI dan konversinya ke/dari DTO. Validasi tetap di Rust:
// helper ini hanya membentuk payload, tidak menolak input.
import type { TxnDetailDto, TxnFormDto, TxnFormKindDto, TxnKindDto } from '@/ipc'

export type FormKind = TxnFormKindDto

export interface SplitRow {
  categoryId: string | null
  amount: number
  memo: string
}

export interface TxnFormState {
  kind: FormKind
  date: string
  accountId: string | null
  toAccountId: string | null
  /** Income/expense: total (negatif = refund). Transfer: positif. */
  amount: number
  split: boolean
  splits: SplitRow[]
  categoryId: string | null
  feeEnabled: boolean
  fee: number
  feeCategoryId: string | null
  payee: string
  note: string
  tags: string[]
}

export interface FormDefaults {
  accountId: string | null
  feeCategoryId: string | null
  today: string
}

export function emptyForm(kind: FormKind, d: FormDefaults): TxnFormState {
  return {
    kind, date: d.today, accountId: d.accountId, toAccountId: null, amount: 0, split: false,
    splits: [], categoryId: null, feeEnabled: false, fee: 0, feeCategoryId: d.feeCategoryId,
    payee: '', note: '', tags: [],
  }
}

const text = (s: string) => (s.trim() ? s.trim() : null)

export function toDto(s: TxnFormState): TxnFormDto {
  const transfer = s.kind === 'transfer'
  const rows: SplitRow[] = s.split ? s.splits : [{ categoryId: s.categoryId, amount: s.amount, memo: '' }]
  // Kategori kosong → splits kosong agar Rust menolak dengan `txn_missing_category`.
  const splits = transfer || rows.some((r) => !r.categoryId)
    ? []
    : rows.map((r) => ({ categoryId: r.categoryId!, amount: r.amount, memo: text(r.memo) }))
  return {
    kind: s.kind,
    date: s.date,
    accountId: s.accountId ?? '',
    toAccountId: transfer ? s.toAccountId : null,
    amount: s.amount,
    splits,
    fee: transfer && s.feeEnabled && s.feeCategoryId ? { amount: s.fee, categoryId: s.feeCategoryId } : null,
    payee: text(s.payee),
    note: text(s.note),
    tags: s.tags,
  }
}

export function isEditable(kind: TxnKindDto): kind is FormKind {
  return kind === 'income' || kind === 'expense' || kind === 'transfer'
}

/** Hanya untuk jenis yang `isEditable`. */
export function fromDetail(d: TxnDetailDto): TxnFormState {
  const kind: FormKind = isEditable(d.kind) ? d.kind : 'expense'
  const s = emptyForm(kind, { accountId: d.postings[0]?.accountId ?? null, feeCategoryId: null, today: d.date })
  Object.assign(s, { payee: d.payee ?? '', note: d.note ?? '', tags: [...d.tags] })
  if (kind === 'transfer') {
    for (const p of d.postings) {
      if (p.categoryId) Object.assign(s, { feeEnabled: true, fee: -p.amount, feeCategoryId: p.categoryId })
      else if (p.amount < 0) s.accountId = p.accountId
      else Object.assign(s, { toAccountId: p.accountId, amount: p.amount })
    }
    return s
  }
  const sign = kind === 'income' ? 1 : -1
  const rows = d.postings
    .filter((p) => p.categoryId)
    .map((p) => ({ categoryId: p.categoryId, amount: p.amount * sign, memo: p.memo ?? '' }))
  s.amount = rows.reduce((m, r) => m + r.amount, 0)
  if (rows.length > 1) Object.assign(s, { split: true, splits: rows })
  else s.categoryId = rows[0]?.categoryId ?? null
  return s
}

export function duplicateOf(d: TxnDetailDto, today: string): TxnFormState {
  return { ...fromDetail(d), date: today }
}

/** Sisa yang belum dialokasikan ke baris split. */
export function splitRemainder(s: TxnFormState): number {
  return s.amount - s.splits.reduce((m, r) => m + r.amount, 0)
}
