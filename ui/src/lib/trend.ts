/**
 * Perubahan relatif periode ini terhadap periode lalu dalam persen bulat (presentasi saja,
 * kedua angka dari backend). `null` bila tidak ada pembanding (periode lalu 0 atau tidak ada).
 */
export function trendPercent(current: number, previous: number | undefined): number | null {
  if (previous === undefined || previous === 0) return null
  return Math.round(((current - previous) * 100) / Math.abs(previous))
}

/** `+12%`, `−4%` (minus U+2212), `0%`: tanda selalu tertulis agar arah tidak bergantung warna. */
export function formatTrend(percent: number): string {
  if (percent > 0) return `+${percent}%`
  if (percent < 0) return `−${Math.abs(percent)}%`
  return '0%'
}
