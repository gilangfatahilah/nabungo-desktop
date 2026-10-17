export const COLORS = ['slate', 'red', 'orange', 'amber', 'green', 'teal', 'sky', 'blue', 'violet', 'pink'] as const

export type ColorName = (typeof COLORS)[number]

// Ditulis lengkap agar Tailwind menemukan class-nya saat scan.
const CLASSES: Record<ColorName, string> = {
  slate: 'bg-slate-500/15 text-slate-600 dark:text-slate-300',
  red: 'bg-red-500/15 text-red-600 dark:text-red-300',
  orange: 'bg-orange-500/15 text-orange-600 dark:text-orange-300',
  amber: 'bg-amber-500/15 text-amber-600 dark:text-amber-300',
  green: 'bg-green-500/15 text-green-600 dark:text-green-300',
  teal: 'bg-teal-500/15 text-teal-600 dark:text-teal-300',
  sky: 'bg-sky-500/15 text-sky-600 dark:text-sky-300',
  blue: 'bg-blue-500/15 text-blue-600 dark:text-blue-300',
  violet: 'bg-violet-500/15 text-violet-600 dark:text-violet-300',
  pink: 'bg-pink-500/15 text-pink-600 dark:text-pink-300',
}

/** Class latar + teks untuk badge ikon; warna tak dikenal → netral. */
export function colorClass(c: string | null): string {
  return (c && CLASSES[c as ColorName]) || 'bg-muted text-muted-foreground'
}
