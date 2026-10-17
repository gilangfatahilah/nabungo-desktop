import type { CategoryDto, CategoryKindDto } from '@/ipc'

export interface CategoryNode {
  category: CategoryDto
  children: CategoryDto[]
}

const bySort = (a: CategoryDto, b: CategoryDto) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name)

/**
 * Pohon 2 level untuk satu jenis. `archived: false` → hanya yang aktif;
 * `archived: true` → hanya yang diarsip (child diarsip dengan parent aktif tampil sebagai node sendiri).
 */
export function buildTree(list: CategoryDto[], kind: CategoryKindDto, opts: { archived: boolean }): CategoryNode[] {
  const ofKind = list.filter((c) => c.kind === kind)
  const shown = ofKind.filter((c) => c.archived === opts.archived)
  const shownIds = new Set(shown.map((c) => c.id))
  const roots = shown.filter((c) => c.parentId === null || !shownIds.has(c.parentId)).sort(bySort)
  return roots.map((category) => ({
    category,
    children: shown.filter((c) => c.parentId === category.id).sort(bySort),
  }))
}

/** Kategori yang boleh menjadi tujuan merge: sejenis, aktif, bukan source. */
export function mergeTargets(list: CategoryDto[], source: CategoryDto): CategoryDto[] {
  return list.filter((c) => c.kind === source.kind && !c.archived && c.id !== source.id).sort(bySort)
}
