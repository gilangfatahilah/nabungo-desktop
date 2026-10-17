<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: HTMLAttributes['class']
}>()
</script>

<template>
  <kbd
    data-slot="kbd"
    :class="cn('border-border text-muted-foreground in-data-[slot=button]:border-current/35 in-data-[slot=button]:text-current/75 in-data-[slot=tooltip-content]:text-background h-5 w-fit min-w-5 gap-1 rounded-sm border px-1.5 font-sans text-meta font-medium [&_svg:not([class*=size-])]:size-3 pointer-events-none inline-flex items-center justify-center select-none', props.class)"
  >
    <slot />
  </kbd>
</template>
