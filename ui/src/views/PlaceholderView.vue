<script setup lang="ts">
defineProps<{ title: string }>()
</script>

<template>
  <section>
    <h1 class="text-2xl font-semibold">{{ title }}</h1>
    <p class="mt-2 text-sm text-muted-foreground">{{ $t('common.comingSoon') }}</p>
  </section>
</template>
