<script setup lang="ts">
import { EllipsisVertical, Plus } from '@lucide/vue'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import EmptyState from '@/components/EmptyState.vue'
import LabelFormDialog from '@/components/LabelFormDialog.vue'
import MergeLabelDialog from '@/components/MergeLabelDialog.vue'
import PageHeader from '@/components/PageHeader.vue'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { labelText, type Label, type LabelKind } from '@/lib/labels'
import { withErrorToast } from '@/lib/notify'
import { cn } from '@/lib/utils'
import { useCategoriesStore } from '@/stores/categories'
import { useLabelsStore } from '@/stores/labels'
import { EMPTY_FILTER, useTransactionsStore } from '@/stores/transactions'

// Tag & Payee (CAT-04). Tab disimpan di query route agar kembali dari Transaksi tetap di tab yang sama.
const store = useLabelsStore()
const categories = useCategoriesStore()
const txns = useTransactionsStore()
const route = useRoute()
const router = useRouter()
const { t } = useI18n()

const KINDS: LabelKind[] = ['tag', 'payee']
const tab = computed<LabelKind>({
  get: () => (route.query.tab === 'payee' ? 'payee' : 'tag'),
  set: (v) => void router.replace({ query: { ...route.query, tab: v } }),
})
const items = computed<Label[]>(() => (tab.value === 'tag' ? store.tags : store.payees))
const plural = computed(() => t(tab.value === 'tag' ? 'labels.tags' : 'labels.payees'))
// Aksi baris tampil saat hover/fokus (§9.4); tetap terlihat selama menunya terbuka.
const reveal = 'opacity-0 group-hover/row:opacity-100 group-focus-within/row:opacity-100 data-[state=open]:opacity-100'

const text = (x: Label) => labelText(tab.value, x.name)
function meta(x: Label): string {
  const usage = t('labels.usage', { n: x.txnCount })
  const catId = 'defaultCategoryId' in x ? x.defaultCategoryId : null
  const cat = catId ? categories.list.find((c) => c.id === catId)?.name : undefined
  return cat ? `${usage} · ${cat}` : usage
}

onMounted(() => {
  // Selalu muat ulang: transaksi di layar lain mengubah jumlah pemakaian dan menambah payee/tag.
  void withErrorToast(() => store.reload())
  if (!categories.loaded) void withErrorToast(() => categories.load())
})

// --- form ---
const formOpen = ref(false)
const editing = ref<Label | undefined>()

function create() {
  editing.value = undefined
  formOpen.value = true
}

function edit(x: Label) {
  editing.value = x
  formOpen.value = true
}

// --- gabung ---
const mergeOpen = ref(false)
const mergeSource = ref<Label | null>(null)

function merge(x: Label) {
  mergeSource.value = x
  mergeOpen.value = true
}

// --- lihat transaksi: filter lain direset ---
async function view(x: Label) {
  await withErrorToast(() =>
    txns.setQuery({ ...EMPTY_FILTER, ...(tab.value === 'tag' ? { tag: x.name } : { payeeId: x.id }) }),
  )
  await router.push('/transactions')
}

// --- hapus (tanpa undo; tidak mengubah saldo) ---
const deleting = ref<Label | null>(null)
const confirmOpen = ref(false)

function remove(x: Label) {
  deleting.value = x
  confirmOpen.value = true
}

function onConfirm() {
  const x = deleting.value
  if (!x) return
  void withErrorToast(() => (tab.value === 'tag' ? store.deleteTag(x.id) : store.deletePayee(x.id)))
}
</script>

<template>
  <PageHeader :title="t('nav.labels')">
    <template #actions>
      <Tabs v-model="tab">
        <TabsList>
          <TabsTrigger v-for="k in KINDS" :key="k" :value="k" class="px-3">{{ t(k === 'tag' ? 'labels.tags' : 'labels.payees') }}</TabsTrigger>
        </TabsList>
      </Tabs>
      <Button @click="create()"><Plus /> {{ t(tab === 'tag' ? 'labels.newTag' : 'labels.newPayee') }}</Button>
    </template>
  </PageHeader>
  <EmptyState v-if="store.loaded && !items.length" :title="t('labels.empty', { what: plural.toLowerCase() })" />
  <ul v-else class="-mx-2 flex flex-col">
    <li
      v-for="x in items" :key="x.id" data-testid="label-row"
      class="group/row flex h-12 items-center gap-3 rounded-lg px-2 hover:bg-muted"
    >
      <span class="min-w-0 flex-1 truncate font-medium">{{ text(x) }}</span>
      <span class="num shrink-0 text-meta text-muted-foreground">{{ meta(x) }}</span>
      <DropdownMenu>
        <DropdownMenuTrigger as-child>
          <Button variant="ghost" size="icon" :class="cn('size-8', reveal)" :aria-label="t('actions.more')" :data-testid="`label-menu-${x.id}`">
            <EllipsisVertical />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem @select="edit(x)">{{ t('actions.edit') }}</DropdownMenuItem>
          <DropdownMenuItem @select="merge(x)">{{ t('labels.merge') }}…</DropdownMenuItem>
          <DropdownMenuItem @select="view(x)">{{ t('labels.viewTxns') }}</DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem variant="destructive" @select="remove(x)">{{ t('actions.delete') }}</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </li>
  </ul>

  <LabelFormDialog v-model:open="formOpen" :kind="tab" :label="editing" />
  <MergeLabelDialog v-if="mergeSource" v-model:open="mergeOpen" :kind="tab" :source="mergeSource" />
  <ConfirmDialog
    v-if="deleting"
    v-model:open="confirmOpen"
    :title="t('labels.deleteTitle', { name: text(deleting) })"
    :description="t('labels.deleteBody', { name: text(deleting), n: deleting.txnCount })"
    :confirm-label="t('actions.delete')"
    destructive
    @confirm="onConfirm"
  />
</template>
