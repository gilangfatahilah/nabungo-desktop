import { afterEach, describe, expect, it, vi } from 'vitest'
import { applyFontScale, FONT_SCALES, isFontScale, readCachedFontScale, rootFontScale, stepFontScale, writeCachedFontScale } from './fontScale'

describe('fontScale', () => {
  afterEach(() => {
    vi.restoreAllMocks()
    localStorage.clear()
    document.documentElement.style.fontSize = ''
  })

  it('pilihan sama dengan Rust', () => {
    expect(FONT_SCALES).toEqual([90, 100, 110, 125, 150])
    expect(isFontScale(110)).toBe(true)
    expect(isFontScale(120)).toBe(false)
    expect(isFontScale('100')).toBe(false)
  })

  it('stepFontScale naik/turun satu langkah dan berhenti di ujung', () => {
    expect(stepFontScale(100, 1)).toBe(110)
    expect(stepFontScale(150, 1)).toBe(150)
    expect(stepFontScale(90, -1)).toBe(90)
    expect(stepFontScale(110, -1)).toBe(100)
    expect(stepFontScale(125, 0)).toBe(100)
  })

  it('cache kosong, sampah, atau tak terbaca → 100', () => {
    expect(readCachedFontScale()).toBe(100)
    localStorage.setItem('nabungo.fontScale', 'abc')
    expect(readCachedFontScale()).toBe(100)
    localStorage.setItem('nabungo.fontScale', '120')
    expect(readCachedFontScale()).toBe(100)
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked')
    })
    expect(readCachedFontScale()).toBe(100)
  })

  it('cache valid terbaca kembali; tulis yang gagal tidak melempar', () => {
    writeCachedFontScale(125)
    expect(readCachedFontScale()).toBe(125)
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('quota')
    })
    expect(() => writeCachedFontScale(110)).not.toThrow()
  })

  it('applyFontScale menyetel font-size <html> dalam persen', () => {
    applyFontScale(125)
    expect(document.documentElement.style.fontSize).toBe('125%')
  })

  it('rootFontScale = font-size html / 16, 1 bila tak terukur', () => {
    expect(rootFontScale()).toBe(1)
    vi.spyOn(window, 'getComputedStyle').mockReturnValue({ fontSize: '20px' } as CSSStyleDeclaration)
    expect(rootFontScale()).toBe(1.25)
  })
})
