<script setup lang="ts">
import { computed } from 'vue'
import { moneyParts } from '@/lib/money'
import { useSettingsStore } from '@/stores/settings'

// Prefiks mata uang dibuat lebih pudar dari angkanya (docs/design/design.md §4).
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

const parts = computed(() => moneyParts(props.amount, settings.locale))
const tone = computed(() => ({ neutral: '', expense: 'text-expense', income: 'text-income' })[props.kind])
const sizeClass = computed(() =>
  props.size ? { sm: 'text-sm', md: 'text-base', lg: 'text-heading font-semibold', display: 'text-display font-semibold' }[props.size] : '',
)
</script>

<template>
  <span class="num whitespace-nowrap" :class="[tone, sizeClass]">{{ parts.sign }}<span
    v-if="currency" data-part="currency" class="font-normal"
  >{{ parts.currency }}&nbsp;</span>{{ parts.number }}</span>
</template>
