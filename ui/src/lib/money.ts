import type { LocaleDto } from '@/ipc'

const MINUS = '−'

export interface MoneyParts {
  /** `−` (U+2212) untuk nominal negatif, kosong selain itu. */
  sign: string
  currency: string
  number: string
}

function formatter(locale: LocaleDto) {
  return new Intl.NumberFormat(locale === 'id' ? 'id-ID' : 'en-US', {
    style: 'currency',
    currency: 'IDR',
    maximumFractionDigits: 0,
  })
}

function assertSafe(amountMinor: number) {
  if (!Number.isSafeInteger(amountMinor)) {
    throw new RangeError(`amount must be a safe integer, got ${amountMinor}`)
  }
}

/**
 * Nominal selalu integer dalam satuan minor (IDR: decimals = 0).
 * Jangan pernah melakukan aritmetika uang dengan float di UI; Rust adalah sumber kebenaran.
 */
export function formatIdr(amountMinor: number, locale: LocaleDto = 'id'): string {
  assertSafe(amountMinor)
  return formatter(locale).format(amountMinor).replace('-', MINUS)
}

/** Bagian nominal untuk tampilan berhierarki (prefiks mata uang dibuat lebih pudar). */
export function moneyParts(amountMinor: number, locale: LocaleDto = 'id'): MoneyParts {
  assertSafe(amountMinor)
  const parts = formatter(locale).formatToParts(Math.abs(amountMinor))
  return {
    sign: amountMinor < 0 ? MINUS : '',
    currency: parts.find((p) => p.type === 'currency')?.value ?? '',
    number: parts
      .filter((p) => p.type === 'integer' || p.type === 'group')
      .map((p) => p.value)
      .join(''),
  }
}
