import { Circle } from '@lucide/vue'
import { describe, expect, it } from 'vitest'
import { KIND_ICONS } from './accounts'
import { colorClass, COLORS } from './colors'
import { ICONS, iconFor } from './icons'

describe('icons & colors', () => {
  it('ikon default kategori dan jenis akun terdaftar', () => {
    for (const n of ['utensils', 'car', 'receipt', 'shopping-bag', 'heart-pulse', 'clapperboard', 'graduation-cap',
      'users', 'landmark', 'circle-ellipsis', 'wallet', 'gift', 'trending-up', 'hand-heart', ...Object.values(KIND_ICONS)]) {
      expect(ICONS[n], n).toBeTruthy()
    }
  })

  it('fallback circle / netral', () => {
    expect(iconFor(null)).toBe(Circle)
    expect(iconFor('tidak-ada')).toBe(Circle)
    expect(colorClass('ungu')).toContain('bg-muted')
    for (const c of COLORS) expect(colorClass(c)).toContain(`bg-${c}-500`)
  })
})
