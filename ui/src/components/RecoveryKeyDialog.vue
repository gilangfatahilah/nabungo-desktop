<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { formatDateInput, todayIso } from '@/lib/date'
import { useSecurityStore } from '@/stores/security'
import { useSettingsStore } from '@/stores/settings'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ recoveryKey: string }>()
const emit = defineEmits<{ done: [] }>()
const { t } = useI18n()
const settings = useSettingsStore()
const security = useSecurityStore()

const saved = ref(false)
watch(open, (v) => v && (saved.value = false))

const groups = computed(() => props.recoveryKey.split('-'))
const today = computed(() => formatDateInput(todayIso(), settings.locale))

async function copy() {
  try {
    await navigator.clipboard.writeText(props.recoveryKey)
    toast.success(t('security.recoveryDialog.copied'))
  } catch (e) {
    toast.error(String(e))
  }
}

function print() {
  window.print()
}

function done() {
  security.recoverySaved()
  open.value = false
  emit('done')
}
</script>

<template>
  <!-- Hanya bisa ditutup lewat "Selesai": kunci ini tidak bisa ditampilkan lagi. -->
  <Dialog v-model:open="open">
    <DialogContent
      class="sm:max-w-lg" :show-close-button="false"
      @escape-key-down="(e: Event) => e.preventDefault()" @interact-outside="(e: Event) => e.preventDefault()"
    >
      <DialogHeader>
        <DialogTitle>{{ t('security.recoveryDialog.title') }}</DialogTitle>
        <DialogDescription>{{ t('security.recoveryDialog.body') }}</DialogDescription>
      </DialogHeader>

      <div
        class="grid grid-cols-4 gap-x-3 gap-y-2 rounded-xl border bg-muted/40 p-4 font-mono text-[15px] tracking-wider"
        :aria-label="t('security.recoveryKey')" role="group"
      >
        <span v-for="(g, i) in groups" :key="i" data-testid="recovery-group">{{ g }}</span>
      </div>
      <p class="text-meta text-muted-foreground">{{ t('security.warning') }}</p>

      <div class="flex gap-2">
        <Button type="button" variant="outline" @click="copy">{{ t('security.recoveryDialog.copy') }}</Button>
        <Button type="button" variant="outline" @click="print">{{ t('security.recoveryDialog.print') }}</Button>
      </div>

      <div class="flex items-start gap-3">
        <Checkbox id="recovery-saved" v-model="saved" class="mt-0.5" />
        <Label for="recovery-saved" class="leading-snug font-normal">{{ t('security.recoveryDialog.saved') }}</Label>
      </div>

      <DialogFooter>
        <Button type="button" :disabled="!saved" @click="done">{{ t('security.recoveryDialog.done') }}</Button>
      </DialogFooter>

      <!-- Hanya tampil saat dicetak (`@media print` di style.css). Di-teleport ke body agar
           tidak ikut `transform` dialog. -->
      <Teleport to="body">
        <section class="print-recovery hidden">
          <h1>{{ t('security.recoveryDialog.printTitle') }}</h1>
          <p>{{ t('security.recoveryDialog.printDate', { date: today }) }}</p>
          <p class="print-recovery-key">{{ recoveryKey }}</p>
          <p>{{ t('security.warning') }}</p>
        </section>
      </Teleport>
    </DialogContent>
  </Dialog>
</template>
