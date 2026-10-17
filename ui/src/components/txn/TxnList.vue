<script setup lang="ts">
import { ArrowDownLeftIcon, ArrowRightIcon, ArrowRightLeftIcon, ArrowUpRightIcon, EllipsisIcon, FlagIcon, PlusIcon, ScaleIcon } from '@lucide/vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { computed, nextTick, onMounted, useTemplateRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import MoneyText from '@/components/MoneyText.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuShortcut, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import type { TxnKindDto, TxnRowDto } from '@/ipc'
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
const categoryName = computed(() => {
  const byId = new Map(categories.list.map((c) => [c.id, c]))
  return (id: string | null) => {
    const c = id ? byId.get(id) : undefined
    if (!c) return ''
    const parent = c.parentId ? byId.get(c.parentId) : undefined
    return parent ? `${parent.name} › ${c.name}` : c.name
  }
})

const ICONS: Record<TxnKindDto, typeof ArrowUpRightIcon> = {
  expense: ArrowUpRightIcon, income: ArrowDownLeftIcon, transfer: ArrowRightLeftIcon,
  adjustment: ScaleIcon, valuation: ScaleIcon, opening: FlagIcon,
}

function categoryLabel(r: TxnRowDto): string {
  if (r.kind === 'transfer') return t('txn.kinds.transfer')
  if (r.kind === 'opening') return t('txn.opening')
  if (r.splitCount > 1) return t('txn.splitCount', { n: r.splitCount })
  return categoryName.value(r.categoryId)
}

const title = (r: TxnRowDto) => r.payee ?? r.note ?? categoryLabel(r)

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
  estimateSize: (i: number) => (items.value[i]?.type === 'date' ? 32 : 56),
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
  <div v-if="store.rows.length === 0 && !store.loading" class="flex flex-1 flex-col items-center justify-center gap-3 py-16 text-muted-foreground">
    <p>{{ t('txn.empty') }}</p>
    <Button data-testid="txn-empty-new" @click="ui.openNewTxn()"><PlusIcon /> {{ t('sidebar.newTransaction') }}</Button>
  </div>
  <div v-else ref="scroller" class="min-h-0 flex-1 overflow-y-auto" role="list" :aria-label="t('nav.transactions')">
    <div :style="virtual ? { height: `${virtualizer.getTotalSize()}px`, position: 'relative' } : undefined">
      <div
        v-for="v in visible"
        :key="v.item.key"
        :data-index="v.index"
        :style="virtual ? { position: 'absolute', top: 0, left: 0, width: '100%', transform: `translateY(${v.start}px)` } : undefined"
      >
        <div v-if="v.item.type === 'date'" class="flex h-8 items-end border-b px-3 pb-1 text-xs font-medium text-muted-foreground">
          {{ formatDateHeader(v.item.date, settings.locale) }}
        </div>
        <div
          v-else
          role="listitem"
          tabindex="0"
          :data-txn-id="v.item.row.id"
          class="flex h-14 items-center gap-3 border-b px-3 outline-none hover:bg-muted/50 focus-visible:bg-muted focus-visible:ring-2 focus-visible:ring-ring/50"
          @keydown="onKey($event, v.item.row)"
          @dblclick="edit(v.item.row)"
        >
          <component :is="ICONS[v.item.row.kind]" class="size-4 shrink-0 text-muted-foreground" :aria-label="t(`txn.kinds.${v.item.row.kind}`)" />
          <div class="flex min-w-0 flex-1 flex-col">
            <span class="truncate text-sm font-medium">{{ title(v.item.row) }}</span>
            <span class="flex min-w-0 items-center gap-2 text-xs text-muted-foreground">
              <span v-if="v.item.row.kind !== 'transfer'" class="truncate">{{ categoryLabel(v.item.row) }}</span>
              <span data-testid="txn-accounts" class="flex items-center gap-1 truncate">
                {{ accountName.get(v.item.row.accountId) }}
                <template v-if="v.item.row.toAccountId">
                  <ArrowRightIcon class="size-3" aria-label="→" /><span class="sr-only">→</span>
                  {{ accountName.get(v.item.row.toAccountId) }}
                </template>
              </span>
              <Badge v-for="tag in v.item.row.tags" :key="tag" variant="secondary" class="h-4 px-1 text-[10px]">#{{ tag }}</Badge>
            </span>
          </div>
          <div class="flex flex-col items-end">
            <MoneyText data-testid="txn-amount" class="text-sm font-medium" :amount="v.item.row.amount" :kind="tone(v.item.row)" />
            <span v-if="v.item.row.fee" class="text-xs text-muted-foreground">{{ t('txn.fee') }}: <MoneyText :amount="v.item.row.fee" /></span>
          </div>
          <MoneyText
            v-if="showRunning && v.item.row.runningBalance !== null"
            data-testid="txn-running"
            class="w-28 text-right text-sm text-muted-foreground"
            :amount="v.item.row.runningBalance"
          />
          <DropdownMenu>
            <DropdownMenuTrigger as-child>
              <Button variant="ghost" size="icon" class="size-8" data-testid="txn-menu" :aria-label="t('actions.more')" tabindex="-1">
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
