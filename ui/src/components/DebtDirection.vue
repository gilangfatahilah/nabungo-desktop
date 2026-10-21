<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'

// Arah akun piutang/hutang (D10): `true` = user yang berutang, saldo disimpan negatif.
const iOwe = defineModel<boolean>({ required: true })
const { t } = useI18n()
const DIRECTIONS = ['owedToMe', 'iOwe'] as const
</script>

<template>
  <Tabs :model-value="iOwe ? 'iOwe' : 'owedToMe'" @update:model-value="iOwe = $event === 'iOwe'">
    <TabsList class="w-full">
      <TabsTrigger v-for="d in DIRECTIONS" :key="d" :value="d" :data-testid="`direction-${d}`" class="flex-1 px-3">
        {{ t(`accounts.direction.${d}`) }}
      </TabsTrigger>
    </TabsList>
  </Tabs>
</template>
