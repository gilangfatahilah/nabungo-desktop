<script setup lang="ts">
// Rata kiri di posisi konten pertama, tanpa ilustrasi (docs/design/design.md §7.7).
defineProps<{ title: string; description?: string }>()
</script>

<template>
  <div class="max-w-prose py-8">
    <p data-part="title" class="font-medium">{{ title }}</p>
    <p v-if="description" data-part="description" class="mt-1 text-muted-foreground">{{ description }}</p>
    <div v-if="$slots.default" class="mt-4">
      <slot />
    </div>
  </div>
</template>
