import { defineStore } from 'pinia'
import { ref } from 'vue'
import { i18n } from '@/i18n'
import {
  getPeriodStartDay,
  getUiSettings,
  setFontScale as ipcSetFontScale,
  setLocale as ipcSetLocale,
  setPeriodStartDay as ipcSetPeriodStartDay,
  setTheme as ipcSetTheme,
  type LocaleDto,
  type ThemeDto,
} from '@/ipc'
import { applyFontScale, isFontScale, readCachedFontScale, stepFontScale as nextFontScale, writeCachedFontScale, type FontScale } from '@/lib/fontScale'
import { applyTheme, resolveTheme } from '@/lib/theme'

const QUERY = '(prefers-color-scheme: dark)'

// jsdom / mode browser tertentu tidak punya matchMedia.
function mediaQuery(): MediaQueryList | null {
  return typeof window !== 'undefined' && typeof window.matchMedia === 'function' ? window.matchMedia(QUERY) : null
}

export const useSettingsStore = defineStore('settings', () => {
  const theme = ref<ThemeDto>('system')
  const locale = ref<LocaleDto>('id')
  const periodStartDay = ref(1)
  const fontScale = ref<FontScale>(readCachedFontScale())
  let listening = false

  function apply() {
    applyTheme(resolveTheme(theme.value, mediaQuery()?.matches ?? false))
  }

  function watchSystem() {
    if (listening) return
    const mq = mediaQuery()
    if (!mq) return
    listening = true
    mq.addEventListener('change', () => {
      if (theme.value === 'system') apply()
    })
  }

  function applyScale() {
    applyFontScale(fontScale.value)
    writeCachedFontScale(fontScale.value)
  }

  function applyLocale() {
    i18n.global.locale.value = locale.value
    if (typeof document !== 'undefined') document.documentElement.lang = locale.value
  }

  async function load() {
    const [s, day] = await Promise.all([getUiSettings(), getPeriodStartDay()])
    theme.value = s.theme
    locale.value = s.locale
    periodStartDay.value = day
    if (isFontScale(s.fontScale)) fontScale.value = s.fontScale
    applyScale()
    applyLocale()
    apply()
    watchSystem()
  }

  async function setTheme(t: ThemeDto) {
    await ipcSetTheme(t)
    theme.value = t
    apply()
  }

  async function setFontScale(s: FontScale) {
    await ipcSetFontScale(s)
    fontScale.value = s
    applyScale()
  }

  async function stepFontScale(step: 1 | -1 | 0) {
    const next = nextFontScale(fontScale.value, step)
    if (next !== fontScale.value) await setFontScale(next)
  }

  async function setLocale(l: LocaleDto) {
    await ipcSetLocale(l)
    locale.value = l
    applyLocale()
  }

  async function setPeriodStartDay(day: number) {
    await ipcSetPeriodStartDay(day)
    periodStartDay.value = day
  }

  // Terapkan tema sistem sedini mungkin agar tidak berkedip terang di sistem gelap.
  apply()
  applyFontScale(fontScale.value)

  return { theme, locale, periodStartDay, fontScale, load, setTheme, setLocale, setPeriodStartDay, setFontScale, stepFontScale }
})
