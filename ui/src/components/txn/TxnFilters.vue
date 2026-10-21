<script setup lang="ts">
import { ListFilterIcon, SearchIcon, XIcon } from '@lucide/vue'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import DateInput from '@/components/DateInput.vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { listTags, parseAmount } from '@/ipc'
import { todayIso } from '@/lib/date'
import { errorMessage, type Translate } from '@/lib/errors'
import { formatIdr } from '@/lib/money'
import { activeFilterCount, type Period, periodOf, periodRange } from '@/lib/txnFilter'
import { withErrorToast } from '@/lib/notify'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useLabelsStore } from '@/stores/labels'
import { useSettingsStore } from '@/stores/settings'
import { type TxnFilter, useTransactionsStore } from '@/stores/transactions'
import CategoryCombobox from './CategoryCombobox.vue'

// Filter disimpan di store (bukan URL). Payee dicari lewat kotak pencarian; filter payeeId
// (dari layar Tag & Payee) hanya tampil sebagai chip.
// Toolbar satu baris (ui-guidelines §8.6): cari, periode, akun selalu terlihat; sisanya di "+ Filter" + chip.
const { t } = useI18n()
const store = useTransactionsStore()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const labels = useLabelsStore()
const settings = useSettingsStore()

// Reka Select tidak menerima nilai kosong; sentinel untuk "Semua".
const ALL = '__all'

const search = ref(store.query.search ?? '')
const minText = ref(store.query.amountMin?.toString() ?? '')
const maxText = ref(store.query.amountMax?.toString() ?? '')
const amountError = ref<string | null>(null)
const tags = ref<string[]>([])

// "Kustom" dipilih tapi tanggal belum diisi: rentang masih cocok dengan preset lain.
const customOpen = ref(false)
const period = computed<Period>(() => (customOpen.value ? 'custom' : periodOf(store.query, todayIso())))
const PERIODS: Period[] = ['all', 'thisMonth', 'last30', 'custom']

function setPeriod(v: unknown) {
  const p = v as Period
  customOpen.value = p === 'custom'
  if (p !== 'custom') apply(periodRange(p, todayIso()))
}

type Chip = { key: 'categoryId' | 'tag' | 'payeeId' | 'amountMin' | 'amountMax'; label: string }
const chips = computed<Chip[]>(() => {
  const q = store.query
  const out: Chip[] = []
  if (q.categoryId) out.push({ key: 'categoryId', label: categories.list.find((c) => c.id === q.categoryId)?.name ?? '' })
  if (q.tag) out.push({ key: 'tag', label: `#${q.tag}` })
  if (q.payeeId) {
    const name = labels.payees.find((p) => p.id === q.payeeId)?.name ?? ''
    out.push({ key: 'payeeId', label: t('txn.filters.payee', { name }) })
  }
  if (q.amountMin !== null) out.push({ key: 'amountMin', label: `≥ ${formatIdr(q.amountMin, settings.locale)}` })
  if (q.amountMax !== null) out.push({ key: 'amountMax', label: `≤ ${formatIdr(q.amountMax, settings.locale)}` })
  return out
})
const CHIP_IDS: Partial<Record<Chip['key'], string>> = { categoryId: 'category', payeeId: 'payee' }
const hasFilter = computed(() => activeFilterCount(store.query) > 0)

const activeAccounts = computed(() => accounts.overview.accounts.filter((a) => !a.archived || a.id === store.query.accountId))

let timer: ReturnType<typeof setTimeout> | undefined
watch(search, (v) => {
  clearTimeout(timer)
  timer = setTimeout(() => apply({ search: v.trim() || null }), 250)
})
onUnmounted(() => clearTimeout(timer))

// Sinkronkan teks lokal saat query diubah dari luar (mis. Reset).
watch(() => store.query, (q) => {
  if ((q.search ?? '') !== search.value.trim()) search.value = q.search ?? ''
  minText.value = q.amountMin?.toString() ?? ''
  maxText.value = q.amountMax?.toString() ?? ''
})

onMounted(() => withErrorToast(async () => {
  tags.value = await listTags()
  if (store.query.payeeId && !labels.loaded) await labels.reload()
}))

function apply(patch: Partial<TxnFilter>) {
  void withErrorToast(() => store.setQuery(patch))
}

async function applyAmount(field: 'amountMin' | 'amountMax', text: string) {
  if (!text.trim()) {
    amountError.value = null
    apply({ [field]: null })
    return
  }
  try {
    const n = Math.abs(await parseAmount(text))
    amountError.value = null
    apply({ [field]: n })
  } catch (e) {
    amountError.value = errorMessage(e, t as unknown as Translate)
  }
}

const select = (v: unknown) => (v === ALL || v == null ? null : String(v))

function focusPanel(e: Event) {
  e.preventDefault()
  ;(e.target as HTMLElement | null)?.focus()
}

function reset() {
  clearTimeout(timer)
  search.value = ''
  amountError.value = null
  customOpen.value = false
  void withErrorToast(() => store.clear())
}
</script>

<template>
  <div class="flex flex-col gap-2 pb-3">
    <div class="flex flex-wrap items-center gap-2">
      <div class="relative w-64">
        <SearchIcon class="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input v-model="search" type="search" class="pl-8" data-testid="filter-search" :placeholder="t('txn.filters.search')" :aria-label="t('txn.filters.search')" />
      </div>
      <Select :model-value="period" @update:model-value="setPeriod">
        <SelectTrigger class="w-44" data-testid="filter-period" :aria-label="t('txn.filters.period')"><SelectValue /></SelectTrigger>
        <SelectContent>
          <SelectItem v-for="p in PERIODS" :key="p" :value="p">{{ t(`txn.filters.periods.${p}`) }}</SelectItem>
        </SelectContent>
      </Select>
      <template v-if="period === 'custom'">
        <DateInput
          nullable class="w-40" data-testid="filter-from" :aria-label="t('txn.filters.from')" :placeholder="t('txn.filters.from')" :model-value="store.query.dateFrom"
          @update:model-value="apply({ dateFrom: $event })"
        />
        <DateInput
          nullable class="w-40" data-testid="filter-to" :aria-label="t('txn.filters.to')" :placeholder="t('txn.filters.to')" :model-value="store.query.dateTo"
          @update:model-value="apply({ dateTo: $event })"
        />
      </template>
      <Select :model-value="store.query.accountId ?? ALL" @update:model-value="apply({ accountId: select($event) })">
        <SelectTrigger class="w-40" data-testid="filter-account" :aria-label="t('txn.account')"><SelectValue /></SelectTrigger>
        <SelectContent>
          <SelectItem :value="ALL">{{ t('txn.filters.allAccounts') }}</SelectItem>
          <SelectItem v-for="a in activeAccounts" :key="a.id" :value="a.id">{{ a.name }}</SelectItem>
        </SelectContent>
      </Select>
      <Popover>
        <PopoverTrigger as-child>
          <Button variant="outline" data-testid="filter-more"><ListFilterIcon /> {{ t('txn.filters.more') }}</Button>
        </PopoverTrigger>
        <!-- Fokus ke panel, bukan ke combobox kategori yang langsung membuka daftarnya; Tab masuk ke field pertama. -->
        <PopoverContent align="start" class="flex w-72 flex-col gap-2 outline-none" @open-auto-focus="focusPanel">
          <Label for="filter-category">{{ t('txn.category') }}</Label>
          <CategoryCombobox id="filter-category"
            :model-value="store.query.categoryId" data-testid="filter-category"
            :placeholder="t('txn.filters.category')" :aria-label="t('txn.filters.category')"
            @update:model-value="(id: string | null) => { if (id !== store.query.categoryId) apply({ categoryId: id }) }"
          />
          <Label id="filter-tag-label" class="mt-1">{{ t('txn.tags') }}</Label>
          <Select :model-value="store.query.tag ?? ALL" @update:model-value="apply({ tag: select($event) })">
            <SelectTrigger aria-labelledby="filter-tag-label" class="w-full" data-testid="filter-tag"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem :value="ALL">{{ t('txn.filters.allTags') }}</SelectItem>
              <SelectItem v-for="tag in tags" :key="tag" :value="tag">#{{ tag }}</SelectItem>
            </SelectContent>
          </Select>
          <Label class="mt-1">{{ t('txn.amount') }}</Label>
          <div class="grid grid-cols-2 gap-2">
            <Input
              v-model="minText" class="num text-right" inputmode="decimal" data-testid="filter-min" :placeholder="t('txn.filters.min')"
              :aria-label="t('txn.filters.min')" :aria-invalid="!!amountError || undefined" @change="applyAmount('amountMin', minText)"
            />
            <Input
              v-model="maxText" class="num text-right" inputmode="decimal" data-testid="filter-max" :placeholder="t('txn.filters.max')"
              :aria-label="t('txn.filters.max')" :aria-invalid="!!amountError || undefined" @change="applyAmount('amountMax', maxText)"
            />
          </div>
          <p v-if="amountError" class="text-meta text-destructive" role="alert">{{ amountError }}</p>
        </PopoverContent>
      </Popover>
      <p class="num ml-auto text-meta text-muted-foreground" data-testid="filter-count">{{ t('txn.filters.count', { n: store.total }) }}</p>
    </div>
    <div v-if="hasFilter" class="flex flex-wrap items-center gap-2">
      <span
        v-for="c in chips" :key="c.key" :data-testid="`chip-${CHIP_IDS[c.key] ?? c.key}`"
        class="inline-flex h-6 items-center gap-1 rounded-full border pr-1 pl-2.5 text-meta"
      >
        {{ c.label }}
        <button
          type="button" class="rounded-full p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
          :aria-label="t('txn.filters.remove', { name: c.label })" @click="apply({ [c.key]: null })"
        >
          <XIcon class="size-3" />
        </button>
      </span>
      <Button variant="ghost" size="sm" class="ml-auto text-muted-foreground" data-testid="filter-reset" @click="reset">{{ t('txn.filters.clearAll') }}</Button>
    </div>
  </div>
</template>
