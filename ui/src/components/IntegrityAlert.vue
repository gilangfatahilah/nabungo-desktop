<script setup lang="ts">
// Peringatan sekali saat startup bila integrity check setelah shutdown tak bersih gagal.
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription,
  AlertDialogFooter, AlertDialogHeader, AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { startupReport } from '@/ipc'
import { withErrorToast } from '@/lib/notify'

const router = useRouter()
const open = ref(false)

onMounted(() =>
  withErrorToast(async () => {
    open.value = (await startupReport()).integrity === 'failed'
  }),
)

function openBackups() {
  open.value = false
  void router.push({ name: 'settings' })
}
</script>

<template>
  <AlertDialog v-model:open="open">
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle>{{ $t('integrity.title') }}</AlertDialogTitle>
        <AlertDialogDescription>{{ $t('integrity.body') }}</AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel>{{ $t('integrity.later') }}</AlertDialogCancel>
        <AlertDialogAction @click="openBackups">{{ $t('integrity.openBackups') }}</AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
