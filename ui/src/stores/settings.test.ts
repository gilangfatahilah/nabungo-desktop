import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { i18n } from '@/i18n'
import { useSettingsStore } from './settings'

describe('settings store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
  })

  it('load tidak error tanpa matchMedia', async () => {
    const s = useSettingsStore()
    await s.load()
    expect(s.theme).toBe('system')
    expect(document.documentElement.classList.contains('dark')).toBe(false)
  })

  it('setTheme dan setLocale memperbarui state, class, dan i18n', async () => {
    const s = useSettingsStore()
    await s.setTheme('dark')
    expect(document.documentElement.classList.contains('dark')).toBe(true)
    await s.setLocale('en')
    expect(i18n.global.locale.value).toBe('en')
    await s.setTheme('light')
    await s.setLocale('id')
  })
})
