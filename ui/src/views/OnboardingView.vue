<script setup lang="ts">
import { computed, ref, useTemplateRef } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import AmountInput from '@/components/AmountInput.vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { completeOnboarding, type AccountKindDto, type LocaleDto } from '@/ipc'
import { ACCOUNT_KINDS, isLiability, openingSign } from '@/lib/accounts'
import { todayIso } from '@/lib/date'
import { withErrorToast } from '@/lib/notify'
import { useSettingsStore } from '@/stores/settings'

const settings = useSettingsStore()
const router = useRouter()
const { t } = useI18n()

const locales: LocaleDto[] = ['id', 'en']
const days = Array.from({ length: 28 }, (_, i) => i + 1)

const day = ref('1')
const kind = ref<AccountKindDto>('bank')
const name = ref('')
const balance = ref(0)
const amountError = ref<string | null>(null)
const busy = ref(false)
const balanceInput = useTemplateRef<{ commit(): Promise<boolean> }>('balanceInput')

const canSubmit = computed(() => name.value.trim() !== '' && !amountError.value && !busy.value)

function onLocale(v: unknown) {
  if (typeof v === 'string' && (locales as string[]).includes(v)) void withErrorToast(() => settings.setLocale(v as LocaleDto))
}

async function submit() {
  if (!canSubmit.value) return
  if (balanceInput.value && !(await balanceInput.value.commit())) return
  busy.value = true
  await withErrorToast(async () => {
    await completeOnboarding({
      locale: settings.locale,
      periodStartDay: Number(day.value),
      accountKind: kind.value,
      account: {
        name: name.value,
        groupId: null,
        includeInNetWorth: true,
        creditLimit: null,
        statementDay: null,
        icon: null,
        color: null,
        openingBalance: balance.value * openingSign(kind.value),
        openingDate: todayIso(),
      },
    })
    settings.periodStartDay = Number(day.value)
    await router.replace('/')
  })
  busy.value = false
}
</script>

<template>
  <main class="mx-auto flex min-h-screen max-w-lg flex-col justify-center gap-8 p-6">
    <header>
      <h1 class="text-2xl font-semibold">{{ t('onboarding.title') }}</h1>
      <p class="mt-1 text-sm text-muted-foreground">{{ t('onboarding.subtitle') }}</p>
    </header>

    <form class="flex flex-col gap-8" @submit.prevent="submit">
      <section class="flex flex-col gap-2">
        <h2 id="ob-locale" class="text-sm font-medium">1. {{ t('onboarding.language') }}</h2>
        <Select :model-value="settings.locale" @update:model-value="onLocale">
          <SelectTrigger aria-labelledby="ob-locale" class="w-full"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem v-for="v in locales" :key="v" :value="v">{{ t(`settings.locales.${v}`) }}</SelectItem>
          </SelectContent>
        </Select>
      </section>

      <section class="flex flex-col gap-2">
        <h2 id="ob-period" class="text-sm font-medium">2. {{ t('onboarding.period') }}</h2>
        <Select v-model="day">
          <SelectTrigger aria-labelledby="ob-period" aria-describedby="ob-period-hint" class="w-full"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem v-for="d in days" :key="d" :value="String(d)">{{ d }}</SelectItem>
          </SelectContent>
        </Select>
        <p id="ob-period-hint" class="text-xs text-muted-foreground">{{ t('settings.periodStartDayHint') }}</p>
      </section>

      <section class="flex flex-col gap-3">
        <h2 class="text-sm font-medium">3. {{ t('onboarding.firstAccount') }}</h2>
        <div class="flex flex-col gap-2">
          <Label for="ob-name">{{ t('onboarding.accountName') }}</Label>
          <Input id="ob-name" v-model="name" :placeholder="t('onboarding.accountNamePlaceholder')" autofocus maxlength="80" />
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div class="flex flex-col gap-2">
            <Label id="ob-kind">{{ t('onboarding.accountKind') }}</Label>
            <Select v-model="kind">
              <SelectTrigger aria-labelledby="ob-kind" class="w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="k in ACCOUNT_KINDS" :key="k" :value="k">{{ t(`accountKinds.${k}`) }}</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="flex flex-col gap-2">
            <Label for="ob-balance">{{ isLiability(kind) ? t('accounts.openingDebt') : t('onboarding.openingBalance') }}</Label>
            <AmountInput id="ob-balance" ref="balanceInput" v-model="balance" @error="amountError = $event" />
          </div>
        </div>
        <p v-if="amountError" class="text-sm text-destructive" role="alert">{{ amountError }}</p>
      </section>

      <Button type="submit" :disabled="!canSubmit">{{ t('onboarding.submit') }}</Button>
    </form>
  </main>
</template>
