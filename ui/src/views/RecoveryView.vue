<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
</script>

<template>
  <main class="flex min-h-svh items-center justify-center p-6">
    <h1 class="text-xl font-semibold">{{ t('nav.recovery') }}</h1>
  </main>
</template>
