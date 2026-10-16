import type { LocaleDto } from '@/ipc'

/**
 * Nominal selalu integer dalam satuan minor (IDR: decimals = 0).
 * Jangan pernah melakukan aritmetika uang dengan float di UI; Rust adalah sumber kebenaran.
 */
export function formatIdr(amountMinor: number, locale: LocaleDto = 'id'): string {
  if (!Number.isSafeInteger(amountMinor)) {
    throw new RangeError(`amount must be a safe integer, got ${amountMinor}`)
  }
  return new Intl.NumberFormat(locale === 'id' ? 'id-ID' : 'en-US', {
    style: 'currency',
    currency: 'IDR',
    maximumFractionDigits: 0,
  }).format(amountMinor)
}
