import { defineStore } from 'pinia'
import { ref } from 'vue'
import { i18n } from '@/i18n'
import { getUiSettings, setLocale as ipcSetLocale, setTheme as ipcSetTheme, type LocaleDto, type ThemeDto } from '@/ipc'
import { applyTheme, resolveTheme } from '@/lib/theme'

const QUERY = '(prefers-color-scheme: dark)'

// jsdom / mode browser tertentu tidak punya matchMedia.
function mediaQuery(): MediaQueryList | null {
  return typeof window !== 'undefined' && typeof window.matchMedia === 'function' ? window.matchMedia(QUERY) : null
}

export const useSettingsStore = defineStore('settings', () => {
  const theme = ref<ThemeDto>('system')
  const locale = ref<LocaleDto>('id')
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

  function applyLocale() {
    i18n.global.locale.value = locale.value
    if (typeof document !== 'undefined') document.documentElement.lang = locale.value
  }

  async function load() {
    const s = await getUiSettings()
    theme.value = s.theme
    locale.value = s.locale
    applyLocale()
    apply()
    watchSystem()
  }

  async function setTheme(t: ThemeDto) {
    await ipcSetTheme(t)
    theme.value = t
    apply()
  }

  async function setLocale(l: LocaleDto) {
    await ipcSetLocale(l)
    locale.value = l
    applyLocale()
  }

  // Terapkan tema sistem sedini mungkin agar tidak berkedip terang di sistem gelap.
  apply()

  return { theme, locale, load, setTheme, setLocale }
})
