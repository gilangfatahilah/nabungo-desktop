<script setup lang="ts">
import { ArrowRightIcon, ArrowRightLeftIcon, EllipsisIcon, FlagIcon, PlusIcon, ScaleIcon, SplitIcon } from '@lucide/vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { type Component, computed, nextTick, onMounted, useTemplateRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import EmptyState from '@/components/EmptyState.vue'
import MoneyText from '@/components/MoneyText.vue'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuShortcut, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Kbd } from '@/components/ui/kbd'
import type { TxnKindDto, TxnRowDto } from '@/ipc'
import { colorClass } from '@/lib/colors'
import { iconFor } from '@/lib/icons'
import { activeFilterCount } from '@/lib/txnFilter'
import { formatDateHeader, groupByDate } from '@/lib/txnList'
import { isEditable } from '@/lib/txnForm'
import { withErrorToast } from '@/lib/notify'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useSettingsStore } from '@/stores/settings'
import { useTransactionsStore } from '@/stores/transactions'
import { useUiStore } from '@/stores/ui'

// `virtual=false` merender semua item tanpa virtualisasi. Hanya untuk test: jsdom tidak
// mengukur layout sehingga virtualizer tidak merender apa pun.
const props = withDefaults(defineProps<{ virtual?: boolean }>(), { virtual: true })

const { t } = useI18n()
const store = useTransactionsStore()
const ui = useUiStore()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const settings = useSettingsStore()

onMounted(() => withErrorToast(async () => {
  await Promise.all([accounts.loaded ? null : accounts.load(), categories.loaded ? null : categories.load()])
}))

const items = computed(() => groupByDate(store.rows))
const showRunning = computed(() => store.query.accountId !== null)

const accountName = computed(() => new Map(accounts.overview.accounts.map((a) => [a.id, a.name])))
const categoryById = computed(() => new Map(categories.list.map((c) => [c.id, c])))
const hasFilter = computed(() => activeFilterCount(store.query) > 0)

/** Nama lengkap "Induk › Anak" untuk tooltip; baris hanya menampilkan anak (§8.5). */
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

const KIND_ICONS: Partial<Record<TxnKindDto, Component>> = {
  transfer: ArrowRightLeftIcon, opening: FlagIcon, adjustment: ScaleIcon, valuation: ScaleIcon,
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

// --- Virtualisasi ---
const scroller = useTemplateRef<HTMLElement>('scroller')
const virtualizer = useVirtualizer(computed(() => ({
  count: items.value.length,
  getScrollElement: () => scroller.value,
  estimateSize: (i: number) => (items.value[i]?.type === 'date' ? 40 : 56),
  getItemKey: (i: number) => items.value[i]?.key ?? i,
  overscan: 10,
})))

const visible = computed(() => props.virtual
  ? virtualizer.value.getVirtualItems().map((v) => ({ item: items.value[v.index]!, index: v.index, start: v.start }))
  : items.value.map((item, index) => ({ item, index, start: 0 })))

watch(() => virtualizer.value.getVirtualItems().at(-1)?.index, (last) => {
  if (props.virtual && last !== undefined && last >= items.value.length - 20) void withErrorToast(() => store.loadMore())
})

// --- Keyboard ---
function rowEls(): HTMLElement[] {
  return [...(scroller.value?.querySelectorAll<HTMLElement>('[data-txn-id]') ?? [])]
}

async function moveFocus(from: HTMLElement, delta: number) {
  const id = from.dataset.txnId
  const idx = items.value.findIndex((i) => i.type === 'row' && i.row.id === id)
  for (let j = idx + delta; j >= 0 && j < items.value.length; j += delta) {
    const target = items.value[j]!
    if (target.type !== 'row') continue
    if (props.virtual) virtualizer.value.scrollToIndex(j)
    await nextTick()
    rowEls().find((el) => el.dataset.txnId === target.row.id)?.focus()
    return
  }
}

function edit(r: TxnRowDto) {
  if (isEditable(r.kind)) ui.openEditTxn(r.id)
}

function duplicate(r: TxnRowDto) {
  if (isEditable(r.kind)) ui.openDuplicateTxn(r.id)
}

function remove(r: TxnRowDto) {
  void withErrorToast(() => store.remove(r.id))
}

function onKey(e: KeyboardEvent, r: TxnRowDto) {
  if (e.target !== e.currentTarget) return
  if (e.key === 'Enter') edit(r)
  else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd') duplicate(r)
  else if (e.key === 'Delete') remove(r)
  else if (e.key === 'ArrowDown') void moveFocus(e.currentTarget as HTMLElement, 1)
  else if (e.key === 'ArrowUp') void moveFocus(e.currentTarget as HTMLElement, -1)
  else return
  e.preventDefault()
}
</script>

<template>
  <template v-if="store.rows.length === 0 && !store.loading">
    <EmptyState v-if="hasFilter" :title="t('txn.emptyFiltered')" class="py-4">
      <Button variant="outline" data-testid="txn-empty-clear" @click="withErrorToast(() => store.clear())">{{ t('txn.filters.clearAll') }}</Button>
    </EmptyState>
    <EmptyState v-else :title="t('txn.empty')" :description="t('txn.emptyHint')" class="py-4">
      <Button data-testid="txn-empty-new" @click="ui.openNewTxn()"><PlusIcon /> {{ t('txn.emptyAction') }} <Kbd class="ml-1">N</Kbd></Button>
    </EmptyState>
  </template>
  <div v-else ref="scroller" class="-mx-2 min-h-0 flex-1 overflow-y-auto px-2" role="list" :aria-label="t('nav.transactions')">
    <div :style="virtual ? { height: `${virtualizer.getTotalSize()}px`, position: 'relative' } : undefined">
      <div
        v-for="v in visible"
        :key="v.item.key"
        :data-index="v.index"
        :style="virtual ? { position: 'absolute', top: 0, left: 0, width: '100%', transform: `translateY(${v.start}px)` } : undefined"
      >
        <div v-if="v.item.type === 'date'" class="flex h-10 items-end px-2 pb-1.5 text-meta font-medium text-muted-foreground">
          {{ formatDateHeader(v.item.date, settings.locale) }}
        </div>
        <div
          v-else
          role="listitem"
          tabindex="0"
          :data-txn-id="v.item.row.id"
          class="group/row flex h-14 items-center gap-3 rounded-lg px-2 outline-none hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
          @keydown="onKey($event, v.item.row)"
          @dblclick="edit(v.item.row)"
        >
          <span data-testid="txn-icon" class="flex size-7 shrink-0 items-center justify-center rounded-md" :class="tile(v.item.row).class">
            <component :is="tile(v.item.row).icon" class="size-4" :aria-label="t(`txn.kinds.${v.item.row.kind}`)" />
          </span>
          <div class="flex min-w-0 flex-1 flex-col">
            <span class="truncate font-medium">{{ title(v.item.row) }}</span>
            <span class="flex min-w-0 items-center gap-3 text-meta text-muted-foreground">
              <span
                v-if="showCategory(v.item.row)" data-testid="txn-category" class="truncate"
                :title="categoryPath(v.item.row.categoryId) || undefined"
              >{{ categoryLabel(v.item.row) }}</span>
              <span data-testid="txn-accounts" class="flex shrink-0 items-center gap-1">
                {{ accountName.get(v.item.row.accountId) }}
                <template v-if="v.item.row.toAccountId">
                  <ArrowRightIcon class="size-3" aria-label="→" /><span class="sr-only">→</span>
                  {{ accountName.get(v.item.row.toAccountId) }}
                </template>
              </span>
              <span v-for="tag in v.item.row.tags" :key="tag" data-testid="txn-tag" class="shrink-0">#{{ tag }}</span>
              <span v-if="v.item.row.fee" class="shrink-0">{{ t('txn.fee') }} <MoneyText :amount="v.item.row.fee" /></span>
            </span>
          </div>
          <div class="flex flex-col items-end">
            <MoneyText data-testid="txn-amount" class="font-medium" :amount="v.item.row.amount" :kind="tone(v.item.row)" />
            <MoneyText
              v-if="showRunning && v.item.row.runningBalance !== null"
              data-testid="txn-running"
              class="text-meta text-muted-foreground"
              :amount="v.item.row.runningBalance"
            />
          </div>
          <DropdownMenu>
            <DropdownMenuTrigger as-child>
              <Button
                variant="ghost" size="icon" class="size-8 opacity-0 group-hover/row:opacity-100 group-focus-within/row:opacity-100 data-[state=open]:opacity-100"
                data-testid="txn-menu" :aria-label="t('actions.more')" tabindex="-1"
              >
                <EllipsisIcon />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem data-testid="txn-menu-edit" :disabled="!isEditable(v.item.row.kind)" @select="edit(v.item.row)">
                {{ t('actions.edit') }}<DropdownMenuShortcut>Enter</DropdownMenuShortcut>
              </DropdownMenuItem>
              <DropdownMenuItem data-testid="txn-menu-duplicate" :disabled="!isEditable(v.item.row.kind)" @select="duplicate(v.item.row)">
                {{ t('actions.duplicate') }}<DropdownMenuShortcut>Ctrl+D</DropdownMenuShortcut>
              </DropdownMenuItem>
              <DropdownMenuItem data-testid="txn-menu-delete" variant="destructive" @select="remove(v.item.row)">
                {{ t('actions.delete') }}<DropdownMenuShortcut>Del</DropdownMenuShortcut>
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
    </div>
  </div>
</template>
