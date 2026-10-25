import { ref } from 'vue'

// Ukuran teks UI (persen). Semua ukuran memakai rem, jadi cukup mengubah font-size <html>.
export const FONT_SCALES = [90, 100, 110, 125, 150] as const
export type FontScale = (typeof FONT_SCALES)[number]

const DEFAULT: FontScale = 100
const CACHE_KEY = 'nabungo.fontScale'

export function isFontScale(v: unknown): v is FontScale {
  return typeof v === 'number' && (FONT_SCALES as readonly number[]).includes(v)
}

export function stepFontScale(cur: FontScale, step: 1 | -1 | 0): FontScale {
  if (step === 0) return DEFAULT
  const i = FONT_SCALES.indexOf(cur) + step
  return FONT_SCALES[Math.min(Math.max(i, 0), FONT_SCALES.length - 1)]
}

/** Naik setiap kali ukuran teks diterapkan; komponen yang menghitung px bergantung padanya. */
export const fontScaleRevision = ref(0)

export function applyFontScale(s: FontScale): void {
  document.documentElement.style.fontSize = `${s}%`
  fontScaleRevision.value++
}

// Cache di localStorage agar layar kunci/onboarding memakai ukuran yang sama sebelum DB terbuka.
// DB tetap sumber kebenaran; storage bisa diblokir, jadi semua akses dibungkus try/catch.
export function readCachedFontScale(): FontScale {
  try {
    const v = Number(localStorage.getItem(CACHE_KEY))
    return isFontScale(v) ? v : DEFAULT
  } catch {
    return DEFAULT
  }
}

export function writeCachedFontScale(s: FontScale): void {
  try {
    localStorage.setItem(CACHE_KEY, String(s))
  } catch {
    // diabaikan: cache hanya kenyamanan
  }
}

// Faktor skala untuk ukuran yang dihitung dalam px (virtualizer, teks SVG). 1 bila tak terukur (jsdom).
export function rootFontScale(): number {
  const px = parseFloat(getComputedStyle(document.documentElement).fontSize)
  return Number.isFinite(px) && px > 0 ? px / 16 : 1
}
