import { ArrowRightLeftIcon, FlagIcon, ScaleIcon, SplitIcon } from '@lucide/vue'
import { type Component, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { TxnKindDto, TxnRowDto } from '@/ipc'
import { colorClass } from '@/lib/colors'
import { iconFor } from '@/lib/icons'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'

const KIND_ICONS: Partial<Record<TxnKindDto, Component>> = {
  transfer: ArrowRightLeftIcon, opening: FlagIcon, adjustment: ScaleIcon, valuation: ScaleIcon,
}

/** Label, tile, dan warna baris transaksi; dipakai daftar Transaksi dan tabel Dasbor (docs/design/design.md §7.5). */
export function useTxnRowDisplay() {
  const { t } = useI18n()
  const accounts = useAccountsStore()
  const categories = useCategoriesStore()

  const accountName = computed(() => new Map(accounts.overview.accounts.map((a) => [a.id, a.name])))
  const categoryById = computed(() => new Map(categories.list.map((c) => [c.id, c])))

  /** Nama lengkap "Induk › Anak" untuk tooltip; baris hanya menampilkan anak. */
  function categoryPath(id: string | null): string {
    const c = id ? categoryById.value.get(id) : undefined
    if (!c) return ''
    const parent = c.parentId ? categoryById.value.get(c.parentId) : undefined
    return parent ? `${parent.name} › ${c.name}` : c.name
  }

  function categoryLabel(r: TxnRowDto): string {
    if (r.kind === 'transfer') return t('txn.kinds.transfer')
    if (r.kind === 'opening') return t('txn.opening')
    if (r.splitCount > 1) return t('txn.splitCount', { n: r.splitCount })
    return (r.categoryId && categoryById.value.get(r.categoryId)?.name) || ''
  }

  /** Tile 28px: ikon & warna kategori; transfer/saldo awal/penyesuaian/split di tile netral. */
  function tile(r: TxnRowDto): { icon: Component; class: string } {
    const kindIcon = KIND_ICONS[r.kind]
    if (kindIcon) return { icon: kindIcon, class: colorClass(null) }
    if (r.splitCount > 1) return { icon: SplitIcon, class: colorClass(null) }
    const c = r.categoryId ? categoryById.value.get(r.categoryId) : undefined
    return { icon: iconFor(c?.icon ?? null), class: colorClass(c?.color ?? null) }
  }

  const title = (r: TxnRowDto) => r.payee ?? r.note ?? categoryLabel(r)
  /** Kategori di baris 2 hanya bila judul bukan kategori itu sendiri. */
  const showCategory = (r: TxnRowDto) => r.kind !== 'transfer' && (r.payee ?? r.note) !== null

  /** Transfer & saldo awal netral; lainnya mengikuti tanda (refund pengeluaran tampil hijau). */
  function tone(r: TxnRowDto): 'neutral' | 'expense' | 'income' {
    if (r.kind === 'transfer' || r.kind === 'opening') return 'neutral'
    return r.amount < 0 ? 'expense' : r.amount > 0 ? 'income' : 'neutral'
  }

  return { accountName, categoryPath, categoryLabel, tile, title, showCategory, tone }
}
