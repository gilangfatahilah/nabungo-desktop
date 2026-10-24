import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { i18n } from '@/i18n'
import { setFontScale } from '@/ipc'
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

  it('load menerapkan fontScale dari backend dan menulis cache', async () => {
    localStorage.setItem('nabungo.fontScale', '150')
    const s = useSettingsStore()
    expect(document.documentElement.style.fontSize).toBe('150%')
    await s.load()
    expect(s.fontScale).toBe(100)
    expect(document.documentElement.style.fontSize).toBe('100%')
    expect(localStorage.getItem('nabungo.fontScale')).toBe('100')
  })

  it('setFontScale dan stepFontScale menyimpan ke backend', async () => {
    const s = useSettingsStore()
    await s.setFontScale(125)
    expect(document.documentElement.style.fontSize).toBe('125%')
    await s.stepFontScale(1)
    expect(s.fontScale).toBe(150)
    await s.stepFontScale(1)
    expect(s.fontScale).toBe(150)
    setActivePinia(createPinia())
    const fresh = useSettingsStore()
    await fresh.load()
    expect(fresh.fontScale).toBe(150)
    await fresh.stepFontScale(0)
    expect(fresh.fontScale).toBe(100)
  })

  it('backend menolak ukuran di luar pilihan', async () => {
    await expect(setFontScale(120)).rejects.toMatchObject({ error: { code: 'invalid_font_scale' } })
  })
})
