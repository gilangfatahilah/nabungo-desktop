<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import { exportCsv, exportJson } from '@/ipc'
import { withErrorToast } from '@/lib/notify'

const { t } = useI18n()
const busy = ref(false)

async function run(fn: () => Promise<string | null>) {
  busy.value = true
  try {
    await withErrorToast(async () => {
      const path = await fn()
      // `null` = dialog dibatalkan.
      if (path) toast.success(t('settings.export.done', { path }))
    })
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <p class="max-w-prose text-muted-foreground">{{ t('settings.export.intro') }}</p>
    <div class="flex flex-wrap gap-2">
      <Button variant="outline" :disabled="busy" @click="run(exportJson)">{{ t('settings.export.json') }}</Button>
      <Button variant="outline" :disabled="busy" @click="run(exportCsv)">{{ t('settings.export.csv') }}</Button>
    </div>
  </div>
</template>
