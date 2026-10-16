import type { ThemeDto } from '@/ipc'

export type ResolvedTheme = 'light' | 'dark'

export function resolveTheme(pref: ThemeDto, systemDark: boolean): ResolvedTheme {
  if (pref === 'system') return systemDark ? 'dark' : 'light'
  return pref
}

export function applyTheme(resolved: ResolvedTheme): void {
  document.documentElement.classList.toggle('dark', resolved === 'dark')
}
