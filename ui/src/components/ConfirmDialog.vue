<script setup lang="ts">
import {
  AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription,
  AlertDialogFooter, AlertDialogHeader, AlertDialogTitle,
} from '@/components/ui/alert-dialog'

const open = defineModel<boolean>('open', { required: true })
withDefaults(
  defineProps<{ title: string; description: string; confirmLabel: string; destructive?: boolean; secondaryLabel?: string }>(),
  { destructive: false, secondaryLabel: undefined },
)
const emit = defineEmits<{ confirm: []; secondary: [] }>()
</script>

<template>
  <AlertDialog v-model:open="open">
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle>{{ title }}</AlertDialogTitle>
        <AlertDialogDescription>{{ description }}</AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel>{{ $t('actions.cancel') }}</AlertDialogCancel>
        <AlertDialogAction v-if="secondaryLabel" variant="outline" @click="emit('secondary')">
          {{ secondaryLabel }}
        </AlertDialogAction>
        <AlertDialogAction :variant="destructive ? 'destructive' : 'default'" @click="emit('confirm')">
          {{ confirmLabel }}
        </AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
</template>
