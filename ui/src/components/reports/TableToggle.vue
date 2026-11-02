<script setup lang="ts">
import { ChevronDown } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'

// Tabel angka di bawah chart Laporan (docs/design/design.md §7.12, §8.8): selalu ada, dilipat default.
defineProps<{ testid?: string }>()
</script>

<template>
  <Collapsible v-slot="{ open }" class="flex flex-col gap-2">
    <CollapsibleTrigger as-child>
      <Button variant="ghost" size="sm" class="self-start text-muted-foreground" :data-testid="testid">
        <ChevronDown :class="['transition-transform', open && 'rotate-180']" aria-hidden="true" />
        {{ $t(open ? 'reports.hideTable' : 'reports.showTable') }}
      </Button>
    </CollapsibleTrigger>
    <CollapsibleContent>
      <slot />
    </CollapsibleContent>
  </Collapsible>
</template>
