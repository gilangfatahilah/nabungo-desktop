<script setup lang="ts">
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet'
import { useUiStore } from '@/stores/ui'

const ui = useUiStore()
</script>

<template>
  <Sheet v-model:open="ui.newTxnOpen">
    <SheetContent side="right">
      <SheetHeader>
        <SheetTitle>{{ $t('txn.new') }}</SheetTitle>
        <SheetDescription>{{ $t('txn.formComingSoon') }}</SheetDescription>
      </SheetHeader>
    </SheetContent>
  </Sheet>
</template>
