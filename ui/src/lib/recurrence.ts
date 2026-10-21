// Teks tampilan jadwal berulang (TRX-06). Aturan pengulangan sendiri ada di Rust.
import type { LocaleDto, RecurrenceDto, RuleDto } from '@/ipc'
import type { Translate } from '@/lib/errors'

const intlLocale = (locale: LocaleDto) => (locale === 'id' ? 'id-ID' : 'en-US')

function localDate(iso: string): Date {
  const [y, m, d] = iso.split('-').map(Number)
  return new Date(y!, m! - 1, d!)
}

/** "Tiap bulan, tgl 25" / "Tiap 2 minggu, Senin" / "Tiap tahun, 29 Feb" / "Tiap hari". */
export function describeRecurrence(rec: RecurrenceDto, nextDate: string, t: Translate, locale: LocaleDto): string {
  const every = rec.interval === 1 ? t(`scheduled.every.${rec.freq}`) : t(`scheduled.everyN.${rec.freq}`, { n: rec.interval })
  const d = localDate(nextDate)
  switch (rec.freq) {
    case 'daily': return every
    case 'weekly': return `${every}, ${new Intl.DateTimeFormat(intlLocale(locale), { weekday: 'long' }).format(d)}`
    case 'monthly': return `${every}, ${t('scheduled.onDay', { d: d.getDate() })}`
    case 'yearly': return `${every}, ${new Intl.DateTimeFormat(intlLocale(locale), { day: 'numeric', month: 'short' }).format(d)}`
  }
}

/** Nama tampilan (K13): payee → kategori pertama → catatan → label jenis. */
export function ruleName(rule: RuleDto, categoryName: (id: string) => string | undefined, t: Translate): string {
  const f = rule.form
  const cat = f.splits[0] ? categoryName(f.splits[0].categoryId) : undefined
  return f.payee?.trim() || cat || f.note?.trim() || t(`txn.kinds.${f.kind}`)
}
