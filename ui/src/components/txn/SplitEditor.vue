<script setup lang="ts">
import { PlusIcon, Trash2Icon } from '@lucide/vue'
import { computed, useTemplateRef } from 'vue'
import { useI18n } from 'vue-i18n'
import AmountInput from '@/components/AmountInput.vue'
import MoneyText from '@/components/MoneyText.vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import type { CategoryKindDto } from '@/ipc'
import type { SplitRow } from '@/lib/txnForm'
import CategoryCombobox from './CategoryCombobox.vue'

const rows = defineModel<SplitRow[]>({ required: true })
const props = defineProps<{ kind: CategoryKindDto; total: number }>()
const emit = defineEmits<{ error: [message: string | null] }>()

const { t } = useI18n()
const amountInputs = useTemplateRef<{ commit(): Promise<boolean> }[]>('amounts')

const remainder = computed(() => props.total - rows.value.reduce((m, r) => m + r.amount, 0))

function add() {
  rows.value = [...rows.value, { categoryId: null, amount: remainder.value, memo: '' }]
}

function removeAt(i: number) {
  rows.value = rows.value.filter((_, n) => n !== i)
}

/** Parse semua nominal baris; dipanggil form sebelum menyimpan. */
async function commit(): Promise<boolean> {
  const ok = await Promise.all((amountInputs.value ?? []).map((r) => r.commit()))
  return ok.every(Boolean)
}

defineExpose({ commit })
</script>

<template>
  <div class="flex flex-col gap-2">
    <div v-for="(row, i) in rows" :key="i" class="grid grid-cols-[1fr_8rem_auto] gap-2">
      <CategoryCombobox
        v-model="row.categoryId"
        :kind="kind"
        :data-testid="`split-category-${i}`"
        :aria-label="t('txn.category')"
      />
      <AmountInput
        ref="amounts"
        v-model="row.amount"
        :data-testid="`split-amount-${i}`"
        :aria-label="t('txn.amount')"
        @error="emit('error', $event)"
      />
      <Button type="button" variant="ghost" size="icon" :aria-label="t('txn.removeSplit')" :disabled="rows.length <= 1" @click="removeAt(i)">
        <Trash2Icon />
      </Button>
      <Input v-model="row.memo" class="col-span-2" :placeholder="t('txn.memo')" :aria-label="t('txn.memo')" />
    </div>
    <div class="flex items-center justify-between text-sm">
      <Button type="button" variant="outline" size="sm" @click="add"><PlusIcon /> {{ t('txn.addSplit') }}</Button>
      <span data-testid="split-remainder" :class="remainder !== 0 ? 'text-destructive' : 'text-muted-foreground'">
        {{ t('txn.remainder') }}: <MoneyText :amount="remainder" />
      </span>
    </div>
  </div>
</template>
