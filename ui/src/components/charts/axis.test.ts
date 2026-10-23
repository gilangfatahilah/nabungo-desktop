import { describe, expect, it } from 'vitest'
import { yTicks } from './axis'

describe('yTicks', () => {
  it('tanpa data hanya tick 0', () => {
    expect(yTicks(0, 1, 4, true)).toEqual([0])
  })

  it('rentang sempit: dibulatkan ke integer tanpa duplikat', () => {
    expect(yTicks(0, 2, 4, false)).toEqual([0, 1, 2])
  })

  it('rentang lebar tetap 5 tick dari min sampai max', () => {
    expect(yTicks(-1000, 7000, 4, false)).toEqual([-1000, 1000, 3000, 5000, 7000])
  })
})
