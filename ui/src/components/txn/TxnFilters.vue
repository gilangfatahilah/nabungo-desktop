<script setup lang="ts">
import { RotateCcwIcon, SearchIcon } from '@lucide/vue'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { listTags, parseAmount } from '@/ipc'
import { errorMessage, type Translate } from '@/lib/errors'
import { withErrorToast } from '@/lib/notify'
import { useAccountsStore } from '@/stores/accounts'
import { type TxnFilter, useTransactionsStore } from '@/stores/transactions'
import CategoryCombobox from './CategoryCombobox.vue'

// Filter disimpan di store (bukan URL). Payee dicari lewat kotak pencarian.
const { t } = useI18n()
const store = useTransactionsStore()
const accounts = useAccountsStore()

// Reka Select tidak menerima nilai kosong; sentinel untuk "Semua".
const ALL = '__all'

const search = ref(store.query.search ?? '')
const minText = ref(store.query.amountMin?.toString() ?? '')
const maxText = ref(store.query.amountMax?.toString() ?? '')
const amountError = ref<string | null>(null)
const tags = ref<string[]>([])

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

function reset() {
  clearTimeout(timer)
  search.value = ''
  amountError.value = null
  apply({ dateFrom: null, dateTo: null, accountId: null, categoryId: null, tag: null, payeeId: null,
    amountMin: null, amountMax: null, search: null })
}
</script>

<template>
  <div class="flex flex-wrap items-end gap-2 border-b p-3">
    <div class="relative min-w-48 flex-1">
      <SearchIcon class="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
      <Input v-model="search" type="search" class="pl-8" data-testid="filter-search" :placeholder="t('txn.filters.search')" :aria-label="t('txn.filters.search')" />
    </div>
    <Input
      type="date" class="w-36" data-testid="filter-from" :aria-label="t('txn.filters.from')" :model-value="store.query.dateFrom ?? ''"
      @change="apply({ dateFrom: ($event.target as HTMLInputElement).value || null })"
    />
    <Input
      type="date" class="w-36" data-testid="filter-to" :aria-label="t('txn.filters.to')" :model-value="store.query.dateTo ?? ''"
      @change="apply({ dateTo: ($event.target as HTMLInputElement).value || null })"
    />
    <Select :model-value="store.query.accountId ?? ALL" @update:model-value="apply({ accountId: select($event) })">
      <SelectTrigger class="w-40" data-testid="filter-account" :aria-label="t('txn.account')"><SelectValue /></SelectTrigger>
      <SelectContent>
        <SelectItem :value="ALL">{{ t('txn.filters.allAccounts') }}</SelectItem>
        <SelectItem v-for="a in activeAccounts" :key="a.id" :value="a.id">{{ a.name }}</SelectItem>
      </SelectContent>
    </Select>
    <CategoryCombobox
      :model-value="store.query.categoryId" class="w-48" data-testid="filter-category"
      :placeholder="t('txn.filters.category')" :aria-label="t('txn.filters.category')"
      @update:model-value="(id: string | null) => { if (id !== store.query.categoryId) apply({ categoryId: id }) }"
    />
    <Select :model-value="store.query.tag ?? ALL" @update:model-value="apply({ tag: select($event) })">
      <SelectTrigger class="w-36" data-testid="filter-tag" :aria-label="t('txn.tags')"><SelectValue /></SelectTrigger>
      <SelectContent>
        <SelectItem :value="ALL">{{ t('txn.filters.allTags') }}</SelectItem>
        <SelectItem v-for="tag in tags" :key="tag" :value="tag">#{{ tag }}</SelectItem>
      </SelectContent>
    </Select>
    <Input
      v-model="minText" class="w-28" inputmode="decimal" data-testid="filter-min" :placeholder="t('txn.filters.min')"
      :aria-label="t('txn.filters.min')" @change="applyAmount('amountMin', minText)"
    />
    <Input
      v-model="maxText" class="w-28" inputmode="decimal" data-testid="filter-max" :placeholder="t('txn.filters.max')"
      :aria-label="t('txn.filters.max')" @change="applyAmount('amountMax', maxText)"
    />
    <Button variant="ghost" data-testid="filter-reset" @click="reset"><RotateCcwIcon /> {{ t('actions.reset') }}</Button>
    <p v-if="amountError" class="w-full text-xs text-destructive" role="alert">{{ amountError }}</p>
  </div>
</template>
