<script setup lang="ts">
import EmptyState from '@/components/EmptyState.vue'
import PageHeader from '@/components/PageHeader.vue'

defineProps<{ title: string }>()
</script>

<template>
  <PageHeader :title="title" />
  <EmptyState :title="$t('common.comingSoon')" class="py-0" />
</template>
