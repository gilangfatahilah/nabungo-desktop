import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { useLabelsStore } from './labels'

describe('labels store (fallback browser)', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('mergeTag memanggil ipc lalu memuat ulang tags', async () => {
    const s = useLabelsStore()
    const a = await s.createTag('sumber-lbl')
    const b = await s.createTag('tujuan-lbl')
    expect(s.tags.map((t) => t.name)).toEqual(expect.arrayContaining(['sumber-lbl', 'tujuan-lbl']))
    await s.mergeTag(a, b)
    expect(s.tags.some((t) => t.id === a)).toBe(false)
    expect(s.tags.some((t) => t.id === b)).toBe(true)
  })

  it('createPayee lalu deletePayee memperbarui payees', async () => {
    const s = useLabelsStore()
    const p = await s.createPayee('Toko LBL', null)
    expect(s.payees.find((x) => x.id === p)?.name).toBe('Toko LBL')
    await s.deletePayee(p)
    expect(s.payees.some((x) => x.id === p)).toBe(false)
  })
})
