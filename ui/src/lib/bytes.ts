import type { LocaleDto } from '@/ipc'

/** Ukuran file ringkas: KB/MB dengan maksimal 1 desimal sesuai locale. */
export function formatBytes(bytes: number, locale: LocaleDto): string {
  const nf = new Intl.NumberFormat(locale === 'id' ? 'id-ID' : 'en-US', { maximumFractionDigits: 1 })
  return bytes >= 1024 * 1024 ? `${nf.format(bytes / (1024 * 1024))} MB` : `${nf.format(Math.max(bytes, 1) / 1024)} KB`
}
