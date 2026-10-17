<script setup lang="ts">
import { computed } from 'vue'
import { formatIdr } from '@/lib/money'
import { useSettingsStore } from '@/stores/settings'

const props = withDefaults(defineProps<{ amount: number; kind?: 'neutral' | 'expense' | 'income' }>(), { kind: 'neutral' })
const settings = useSettingsStore()

const tone = computed(() => ({ neutral: '', expense: 'text-expense', income: 'text-income' })[props.kind])
</script>

<template>
  <span class="tabular-nums" :class="tone">{{ formatIdr(amount, settings.locale) }}</span>
</template>
