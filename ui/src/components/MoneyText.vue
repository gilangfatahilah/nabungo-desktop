<script setup lang="ts">
import { computed } from 'vue'
import { i18n } from '@/i18n'
import { moneyParts } from '@/lib/money'
import { MASK } from '@/lib/privacy'
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'

// Prefiks mata uang dibuat lebih pudar dari angkanya (docs/design/design.md §4).
// Mode sembunyikan nominal: angka, tanda, dan nada warna diganti agar defisit tidak terbaca.
const props = withDefaults(
  defineProps<{
    amount: number
    kind?: 'neutral' | 'expense' | 'income'
    size?: 'sm' | 'md' | 'lg' | 'display'
    currency?: boolean
  }>(),
  { kind: 'neutral', size: undefined, currency: true },
)
const settings = useSettingsStore()
const ui = useUiStore()

const parts = computed(() => {
  const p = moneyParts(props.amount, settings.locale)
  return ui.amountsHidden ? { ...p, sign: '', number: MASK } : p
})
const tone = computed(() =>
  ui.amountsHidden ? '' : { neutral: '', expense: 'text-expense', income: 'text-income' }[props.kind],
)
const sizeClass = computed(() =>
  props.size ? { sm: 'text-sm', md: 'text-base', lg: 'text-heading font-semibold', display: 'text-display font-semibold' }[props.size] : '',
)
</script>

<template>
  <span
    class="num whitespace-nowrap" :class="[tone, sizeClass]"
    :data-hidden="ui.amountsHidden || undefined" :aria-label="ui.amountsHidden ? i18n.global.t('privacy.hiddenAmount') : undefined"
  >{{ parts.sign }}<span
    v-if="currency" data-part="currency" class="font-normal"
  >{{ parts.currency }}&nbsp;</span>{{ parts.number }}</span>
</template>
