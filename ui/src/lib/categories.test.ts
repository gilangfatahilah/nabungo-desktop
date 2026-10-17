import { describe, expect, it } from 'vitest'
import type { CategoryDto } from '@/ipc'
import { buildTree, mergeTargets } from './categories'

function cat(id: string, kind: CategoryDto['kind'], parentId: string | null, sortOrder: number, extra: Partial<CategoryDto> = {}): CategoryDto {
  return { id, parentId, kind, name: id, icon: null, color: null, sortOrder, archived: false, inUse: false, ...extra }
}

const list: CategoryDto[] = [
  cat('transport', 'expense', null, 2),
  cat('makan', 'expense', null, 1),
  cat('kopi', 'expense', 'makan', 2),
  cat('resto', 'expense', 'makan', 1),
  cat('jajan', 'expense', 'makan', 3, { archived: true }),
  cat('gaji', 'income', null, 1),
  cat('lama', 'expense', null, 3, { archived: true }),
]

describe('buildTree', () => {
  it('memisah jenis, child di bawah parent, urut sortOrder, arsip disembunyikan', () => {
    const t = buildTree(list, 'expense', { archived: false })
    expect(t.map((n) => n.category.id)).toEqual(['makan', 'transport'])
    expect(t[0]!.children.map((c) => c.id)).toEqual(['resto', 'kopi'])
    expect(buildTree(list, 'income', { archived: false }).map((n) => n.category.id)).toEqual(['gaji'])
  })

  it('archived: true → hanya kategori diarsip', () => {
    const t = buildTree(list, 'expense', { archived: true })
    expect(t.map((n) => n.category.id)).toEqual(['jajan', 'lama'])
  })
})

describe('mergeTargets', () => {
  it('sejenis, aktif, bukan source', () => {
    const ids = mergeTargets(list, list.find((c) => c.id === 'kopi')!).map((c) => c.id)
    expect(ids).toContain('resto')
    expect(ids).toContain('makan')
    expect(ids).not.toContain('kopi')
    expect(ids).not.toContain('gaji')
    expect(ids).not.toContain('jajan')
    expect(ids).not.toContain('lama')
  })
})
