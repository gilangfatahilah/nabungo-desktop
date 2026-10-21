<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import AmountInput from '@/components/AmountInput.vue'
import DateInput from '@/components/DateInput.vue'
import DebtDirection from '@/components/DebtDirection.vue'
import MoneyText from '@/components/MoneyText.vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import type { AccountDto } from '@/ipc'
import { openingSign } from '@/lib/accounts'
import { todayIso } from '@/lib/date'
import { errorMessage, type Translate } from '@/lib/errors'
import { formatIdr } from '@/lib/money'
import { useAccountsStore } from '@/stores/accounts'
import { useSettingsStore } from '@/stores/settings'

// Rekonsiliasi (ACC-04): user memasukkan saldo riil per tanggal; backend mencatat selisihnya
// sebagai penyesuaian dan menandai transaksi sampai tanggal itu.
const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ account: AccountDto }>()

const store = useAccountsStore()
const settings = useSettingsStore()
const { t } = useI18n()
const tr = t as unknown as Translate

const value = ref(0)
const date = ref(todayIso())
const iOwe = ref(false)
const error = ref<string | null>(null)
const busy = ref(false)

const dateInput = useTemplateRef<{ commit(): boolean }>('dateInput')
const valueInput = useTemplateRef<{ commit(): Promise<boolean> }>('valueInput')

const isReceivable = computed(() => props.account.kind === 'receivable')

watch(open, (v) => {
  if (!v) return
  // Utang diketik positif seperti di form akun; arah piutang/hutang dari tanda saldo sekarang.
  iOwe.value = isReceivable.value && props.account.balance < 0
  value.value = props.account.balance * openingSign(props.account.kind, iOwe.value)
  date.value = todayIso()
  error.value = null
}, { immediate: true })

async function submit() {
  if (busy.value) return
  const ok = await Promise.all([dateInput.value?.commit() ?? true, valueInput.value?.commit() ?? true])
  if (!ok.every(Boolean) || error.value) return
  busy.value = true
  try {
    const r = await store.reconcile(props.account.id, date.value, value.value * openingSign(props.account.kind, iOwe.value))
    toast.success(r.difference === 0
      ? t('reconcile.matched')
      : t('reconcile.adjusted', { amount: formatIdr(r.difference, settings.locale) }))
    open.value = false
  } catch (e) {
    error.value = errorMessage(e, tr)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <DialogTitle>{{ t('reconcile.title', { name: account.name }) }}</DialogTitle>
        <DialogDescription class="flex items-center justify-between gap-3">
          <span>{{ t('reconcile.recorded') }}</span>
          <MoneyText :amount="account.balance" />
        </DialogDescription>
      </DialogHeader>
      <form data-testid="reconcile-form" class="flex flex-col gap-4" novalidate @submit.prevent="submit">
        <DebtDirection v-if="isReceivable" v-model="iOwe" />
        <div class="grid grid-cols-2 gap-3">
          <div class="flex flex-col gap-2">
            <Label for="rec-balance">{{ t('reconcile.real') }}</Label>
            <div class="relative">
              <span class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-muted-foreground">Rp</span>
              <AmountInput id="rec-balance" ref="valueInput" v-model="value" class="num pl-10 text-right" autofocus @error="error = $event" />
            </div>
          </div>
          <div class="flex flex-col gap-2">
            <Label for="rec-date">{{ t('reconcile.date') }}</Label>
            <DateInput id="rec-date" ref="dateInput" v-model="date" />
          </div>
        </div>
        <p v-if="error" class="text-sm text-destructive" role="alert">{{ error }}</p>
        <DialogFooter>
          <Button type="button" variant="ghost" @click="open = false">{{ t('actions.cancel') }}</Button>
          <Button type="submit" :disabled="busy">{{ t('reconcile.submit') }}</Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
