<script setup lang="ts">
import { ArrowRightIcon, CircleCheckIcon, PaperclipIcon } from '@lucide/vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { computed, nextTick, onMounted, useTemplateRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import MoneyText from '@/components/MoneyText.vue'
import { useTxnRowActions } from '@/composables/useTxnRowActions'
import { useTxnRowDisplay } from '@/composables/useTxnRowDisplay'
import { formatDateHeader, groupByDate } from '@/lib/txnList'
import { withErrorToast } from '@/lib/notify'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useSettingsStore } from '@/stores/settings'
import { useTransactionsStore } from '@/stores/transactions'
import TxnEmpty from './TxnEmpty.vue'
import TxnRowMenu from './TxnRowMenu.vue'

// `virtual=false` merender semua item tanpa virtualisasi. Hanya untuk test: jsdom tidak
// mengukur layout sehingga virtualizer tidak merender apa pun.
const props = withDefaults(defineProps<{ virtual?: boolean }>(), { virtual: true })

const { t } = useI18n()
const store = useTransactionsStore()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const settings = useSettingsStore()

onMounted(() => withErrorToast(async () => {
  await Promise.all([accounts.loaded ? null : accounts.load(), categories.loaded ? null : categories.load()])
}))

const items = computed(() => groupByDate(store.rows))
const showRunning = computed(() => store.query.accountId !== null)

const { accountName, categoryPath, categoryLabel, tile, title, showCategory, tone } = useTxnRowDisplay()
const { edit, onKey } = useTxnRowActions()

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

/** Header tanggal yang menempel (design.md §8.2): tanggal milik item teratas yang terlihat setelah di-scroll. */
const stickyDate = computed(() => {
  const offset = virtualizer.value.scrollOffset ?? 0
  if (!props.virtual || offset <= 0) return null
  const top = virtualizer.value.getVirtualItems().find((v) => v.end > offset)
  for (let i = top?.index ?? -1; i >= 0; i--) {
    const item = items.value[i]!
    if (item.type === 'date') return item.date
  }
  return null
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
</script>

<template>
  <TxnEmpty v-if="store.rows.length === 0 && !store.loading" />
  <div v-else ref="scroller" class="-mx-2 min-h-0 flex-1 overflow-y-auto px-2" role="list" :aria-label="t('nav.transactions')">
    <!-- Tinggi 0 agar tidak menggeser daftar; isinya menutupi header tanggal yang sudah lewat. -->
    <div v-if="stickyDate" class="sticky top-0 z-10 h-0" aria-hidden="true" data-testid="txn-sticky-date">
      <div class="flex h-10 items-end bg-background px-2 pb-1.5 text-meta font-medium text-muted-foreground">
        {{ formatDateHeader(stickyDate, settings.locale) }}
      </div>
    </div>
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
          @keydown="onKey($event, v.item.row, moveFocus)"
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
              <PaperclipIcon
                v-if="v.item.row.attachmentCount > 0" data-testid="txn-attachment" class="size-3.5 shrink-0"
                role="img" :aria-label="t('txn.hasAttachment')"
              ><title>{{ t('txn.hasAttachment') }}</title></PaperclipIcon>
              <CircleCheckIcon
                v-if="v.item.row.reconciled" data-testid="txn-reconciled" class="size-3.5 shrink-0"
                role="img" :aria-label="t('txn.reconciled')"
              ><title>{{ t('txn.reconciled') }}</title></CircleCheckIcon>
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
          <TxnRowMenu :row="v.item.row" />
        </div>
      </div>
    </div>
  </div>
</template>
