<script setup lang="ts">
import { Plus, X } from '@lucide/vue'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { NativeSelect, NativeSelectOption } from '@/components/ui/native-select'
import type { AccountDto } from '@/ipc'
import { errorMessage, type Translate } from '@/lib/errors'
import { formatBp, parsePercent } from '@/lib/percent'
import { cn } from '@/lib/utils'
import { useAllocationStore } from '@/stores/allocation'
import { useSettingsStore } from '@/stores/settings'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ account: AccountDto }>()

const store = useAllocationStore()
const settings = useSettingsStore()
const { t } = useI18n()

const rows = ref<{ bucketId: string; percent: string }[]>([])
const error = ref<string | null>(null)
const busy = ref(false)

const buckets = computed(() => {
  const all = store.overview?.buckets ?? []
  const name = (id: string | null) => all.find((b) => b.id === id)?.name
  return all.map((b) => ({ id: b.id, label: b.parentId ? `${name(b.parentId)} › ${b.name}` : b.name }))
})
// Sisa hanya umpan balik saat mengetik; validasi Σ porsi tetap di Rust.
const remaining = computed(() => 10_000 - rows.value.reduce((m, r) => m + (parsePercent(r.percent) ?? 0), 0))

watch(
  open,
  (v) => {
    if (!v) return
    const mine = (store.overview?.mappings ?? []).filter((m) => m.accountId === props.account.id)
    rows.value = mine.length
      ? mine.map((m) => ({ bucketId: m.bucketId, percent: formatBp(m.shareBp, settings.locale).replace('%', '') }))
      : [{ bucketId: '', percent: '' }]
    error.value = null
  },
  { immediate: true },
)

async function submit() {
  if (busy.value) return
  const filled = rows.value.filter((r) => r.bucketId !== '' || r.percent.trim() !== '')
  const shares = filled.map((r) => ({ bucketId: r.bucketId, shareBp: parsePercent(r.percent) }))
  if (shares.some((s) => s.bucketId === '' || s.shareBp === null)) {
    error.value = t('allocation.percentInvalid')
    return
  }
  busy.value = true
  error.value = null
  try {
    await store.setAccountAllocation(props.account.id, shares.map((s) => ({ bucketId: s.bucketId, shareBp: s.shareBp! })))
    open.value = false
  } catch (e) {
    error.value = errorMessage(e, t as unknown as Translate)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-lg">
      <DialogHeader>
        <DialogTitle>{{ t('allocation.mapTitle', { name: account.name }) }}</DialogTitle>
        <DialogDescription>{{ t('allocation.mapHint') }}</DialogDescription>
      </DialogHeader>
      <form data-testid="allocation-form" class="flex flex-col gap-3" novalidate @submit.prevent="submit" @keydown.ctrl.enter.prevent="submit">
        <div v-for="(row, i) in rows" :key="i" class="flex items-center gap-2">
          <NativeSelect
            v-model="row.bucketId" :data-testid="`share-bucket-${i}`" :aria-label="t('allocation.bucket')"
            class="min-w-0 flex-1 [&>select]:w-full"
          >
            <NativeSelectOption value="" disabled>{{ t('allocation.chooseBucket') }}</NativeSelectOption>
            <NativeSelectOption v-for="b in buckets" :key="b.id" :value="b.id">{{ b.label }}</NativeSelectOption>
          </NativeSelect>
          <Input
            v-model="row.percent" :data-testid="`share-percent-${i}`" :aria-label="t('allocation.share')"
            inputmode="decimal" class="num w-20 text-right"
          />
          <span class="text-muted-foreground">%</span>
          <Button type="button" variant="ghost" size="icon" class="size-8" :aria-label="t('allocation.removeRow')" @click="rows.splice(i, 1)">
            <X />
          </Button>
        </div>
        <div class="flex items-center justify-between">
          <Button type="button" variant="ghost" size="sm" data-testid="share-add" @click="rows.push({ bucketId: '', percent: '' })">
            <Plus /> {{ t('allocation.addRow') }}
          </Button>
          <span :class="cn('num text-meta', remaining < 0 ? 'text-destructive' : 'text-muted-foreground')">
            {{ t('allocation.remaining', { percent: formatBp(remaining, settings.locale) }) }}
          </span>
        </div>
        <p v-if="error" class="text-sm text-destructive" role="alert">{{ error }}</p>
        <DialogFooter>
          <Button type="button" variant="ghost" @click="open = false">{{ t('actions.cancel') }}</Button>
          <Button type="submit" :disabled="busy">
            {{ t('actions.save') }} <kbd class="ml-1 text-xs opacity-70">Ctrl+Enter</kbd>
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
