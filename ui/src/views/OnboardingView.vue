<script setup lang="ts">
import { ref, useTemplateRef, watch } from 'vue'
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
const nameError = ref<string | null>(null)
const busy = ref(false)
const balanceInput = useTemplateRef<{ commit(): Promise<boolean> }>('balanceInput')

// Tombol tetap aktif; validasi tampil di field (docs/design/ui-guidelines.md §9.7).
watch(name, () => (nameError.value = null))

function onLocale(v: unknown) {
  if (typeof v === 'string' && (locales as string[]).includes(v)) void withErrorToast(() => settings.setLocale(v as LocaleDto))
}

async function submit() {
  if (busy.value) return
  if (name.value.trim() === '') {
    nameError.value = t('errors.name_empty')
    document.getElementById('ob-name')?.focus()
    return
  }
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
  <!-- Satu kolom 440px, rata kiri, di tengah layar (docs/design/ui-guidelines.md §9.7). -->
  <main class="mx-auto flex min-h-screen w-full max-w-[440px] flex-col justify-center gap-8 px-4 py-10">
    <header>
      <h1 class="text-title font-semibold tracking-tight">{{ t('onboarding.title') }}</h1>
      <p class="mt-1 text-meta text-muted-foreground">{{ t('onboarding.subtitle') }}</p>
    </header>

    <form class="flex flex-col gap-8" novalidate @submit.prevent="submit">
      <section class="flex flex-col gap-2">
        <h2 id="ob-locale" class="font-medium">1. {{ t('onboarding.language') }}</h2>
        <Select :model-value="settings.locale" @update:model-value="onLocale">
          <SelectTrigger aria-labelledby="ob-locale" class="w-full"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem v-for="v in locales" :key="v" :value="v">{{ t(`settings.locales.${v}`) }}</SelectItem>
          </SelectContent>
        </Select>
      </section>

      <section class="flex flex-col gap-2">
        <h2 id="ob-period" class="font-medium">2. {{ t('onboarding.period') }}</h2>
        <Select v-model="day">
          <SelectTrigger aria-labelledby="ob-period" aria-describedby="ob-period-hint" class="w-full" data-testid="ob-period">
            <SelectValue>{{ t('onboarding.periodDay', { day }) }}</SelectValue>
          </SelectTrigger>
          <SelectContent class="max-h-72">
            <SelectItem v-for="d in days" :key="d" :value="String(d)">{{ t('onboarding.periodDay', { day: d }) }}</SelectItem>
          </SelectContent>
        </Select>
        <p id="ob-period-hint" class="text-meta text-muted-foreground">{{ t('settings.periodStartDayHint') }}</p>
      </section>

      <section class="flex flex-col gap-4">
        <h2 class="font-medium">3. {{ t('onboarding.firstAccount') }}</h2>
        <div class="flex flex-col gap-2">
          <Label for="ob-name">{{ t('onboarding.accountName') }}</Label>
          <Input
            id="ob-name" v-model="name" :placeholder="t('onboarding.accountNamePlaceholder')" autofocus maxlength="80"
            :aria-invalid="nameError ? 'true' : undefined" :aria-describedby="nameError ? 'ob-name-error' : undefined"
          />
          <p v-if="nameError" id="ob-name-error" class="text-meta text-destructive" role="alert">{{ nameError }}</p>
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
            <AmountInput id="ob-balance" ref="balanceInput" v-model="balance" class="num text-right" @error="amountError = $event" />
          </div>
        </div>
        <p v-if="amountError" class="text-meta text-destructive" role="alert">{{ amountError }}</p>
      </section>

      <Button type="submit" size="lg" :disabled="busy">{{ t('onboarding.submit') }}</Button>
    </form>
  </main>
</template>
