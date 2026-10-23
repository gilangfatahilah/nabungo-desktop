import { describe, expect, it } from 'vitest'
import { monotonePath } from './curve'

describe('monotonePath', () => {
  it('kosong, satu, dan dua titik memakai garis lurus', () => {
    expect(monotonePath([])).toBe('')
    expect(monotonePath([{ x: 0, y: 5 }])).toBe('M0,5')
    expect(monotonePath([{ x: 0, y: 5 }, { x: 10, y: 0 }])).toBe('M0,5 L10,0')
  })

  it('satu segmen kubik per pasangan titik, melewati semua titik', () => {
    const d = monotonePath([{ x: 0, y: 0 }, { x: 10, y: 10 }, { x: 20, y: 0 }])
    expect(d.match(/C/g)).toHaveLength(2)
    expect(d.startsWith('M0,0')).toBe(true)
    expect(d).toContain(' 10,10 ')
    expect(d.endsWith(' 20,0')).toBe(true)
  })

  it('titik puncak lokal punya tangen datar (tidak overshoot)', () => {
    const d = monotonePath([{ x: 0, y: 0 }, { x: 10, y: 10 }, { x: 20, y: 0 }])
    // Titik kontrol di kiri-kanan puncak sejajar puncak (y = 10).
    expect(d).toContain('C3.3333333333333335,3.3333333333333335 6.666666666666666,10 10,10')
    expect(d).toContain('C13.333333333333334,10')
  })
})
