<script setup lang="ts">
// Header wajib setiap halaman (docs/design/design.md §6.4).
defineProps<{ title: string }>()
</script>

<template>
  <header class="flex items-start justify-between gap-4 pb-6">
    <div class="min-w-0">
      <h1 class="text-title font-semibold tracking-tight">{{ title }}</h1>
      <p v-if="$slots.default" data-part="summary" class="mt-1 text-meta text-muted-foreground">
        <slot />
      </p>
    </div>
    <div v-if="$slots.actions" data-part="actions" class="flex shrink-0 items-center gap-2">
      <slot name="actions" />
    </div>
  </header>
</template>
