<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { appInfo } from '@/ipc'

const version = ref<string>('')

onMounted(async () => {
  version.value = (await appInfo()).version
})
</script>

<template>
  <Card class="max-w-sm">
    <CardHeader>
      <CardTitle>{{ $t('dashboard.netWorth') }}</CardTitle>
    </CardHeader>
    <CardContent class="text-sm text-muted-foreground">core v{{ version }}</CardContent>
  </Card>
</template>
