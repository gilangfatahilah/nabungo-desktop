<script setup lang="ts">
// Baris pertama konten: ringkasan di kiri, aksi halaman di kanan. Judul ada di SiteHeader
// (docs/design/design.md §6.4).
</script>

<template>
  <div v-if="$slots.default || $slots.actions" class="flex items-center justify-between gap-4 pb-4 md:pb-6">
    <p v-if="$slots.default" data-part="summary" class="min-w-0 text-meta text-muted-foreground">
      <slot />
    </p>
    <div v-if="$slots.actions" data-part="actions" class="ml-auto flex shrink-0 items-center gap-2">
      <slot name="actions" />
    </div>
  </div>
</template>
