<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import EmptyState from '@/components/EmptyState.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { backupNow, listBackups, restoreBackup, restoreBackupFromFile, type BackupDto } from '@/ipc'
import { formatTimestamp } from '@/lib/date'
import { withErrorToast } from '@/lib/notify'
import { reloadApp } from '@/lib/reload'
import { useSettingsStore } from '@/stores/settings'

const { t } = useI18n()
const settings = useSettingsStore()

const backups = ref<BackupDto[]>([])
const busy = ref(false)
const confirmOpen = ref(false)
// `null` = pulihkan dari file; BackupDto = pulihkan backup dari daftar. Tidak direset saat
// dialog ditutup, karena dialog bisa tertutup sebelum `confirm` diproses.
const pending = ref<BackupDto | null>(null)

function askRestore(b: BackupDto | null) {
  pending.value = b
  confirmOpen.value = true
}

const when = (b: BackupDto) => formatTimestamp(b.createdAt, settings.locale)
const confirmTitle = computed(() =>
  pending.value ? t('settings.backup.restoreTitle', { time: when(pending.value) }) : t('settings.backup.restoreFileTitle'),
)

function formatSize(bytes: number): string {
  const nf = new Intl.NumberFormat(settings.locale === 'id' ? 'id-ID' : 'en-US', { maximumFractionDigits: 1 })
  return bytes >= 1024 * 1024 ? `${nf.format(bytes / (1024 * 1024))} MB` : `${nf.format(Math.max(bytes, 1) / 1024)} KB`
}

async function run(fn: () => Promise<unknown>) {
  busy.value = true
  try {
    await withErrorToast(fn)
  } finally {
    busy.value = false
  }
}

const load = () => run(async () => (backups.value = await listBackups()))

onMounted(load)

function onBackupNow() {
  void run(async () => {
    const path = await backupNow()
    if (path) toast.success(t('settings.backup.saved', { path }))
    backups.value = await listBackups()
  })
}

function onConfirm() {
  const target = pending.value
  confirmOpen.value = false
  void run(async () => {
    const restored = target ? (await restoreBackup(target.fileName), true) : await restoreBackupFromFile()
    if (!restored) return
    toast.success(t('settings.backup.restored'))
    reloadApp()
  })
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <p class="max-w-prose text-muted-foreground">{{ t('settings.backup.intro') }}</p>
    <div class="flex flex-wrap gap-2">
      <Button :disabled="busy" @click="onBackupNow">{{ t('settings.backup.backupNow') }}</Button>
      <Button variant="outline" :disabled="busy" @click="askRestore(null)">{{ t('settings.backup.restoreFromFile') }}</Button>
    </div>

    <EmptyState v-if="backups.length === 0" :title="t('settings.backup.empty')" :description="t('settings.backup.emptyHint')" />
    <div v-else class="max-w-2xl" role="table" :aria-label="t('settings.sections.backup')">
      <div role="row" class="grid h-8 grid-cols-[1fr_10rem_6rem_auto] items-center gap-3 border-b px-2 text-meta text-muted-foreground">
        <span role="columnheader">{{ t('settings.backup.time') }}</span>
        <span role="columnheader">{{ t('settings.backup.kind') }}</span>
        <span role="columnheader" class="text-right">{{ t('settings.backup.size') }}</span>
        <span role="columnheader" class="w-[4.5rem]" />
      </div>
      <div
        v-for="b in backups" :key="b.fileName" role="row" data-testid="backup-row"
        class="grid h-12 grid-cols-[1fr_10rem_6rem_auto] items-center gap-3 border-b px-2"
      >
        <span role="cell" :title="b.fileName">{{ when(b) }}</span>
        <span role="cell"><Badge variant="secondary">{{ t(`settings.backup.kinds.${b.kind}`) }}</Badge></span>
        <span role="cell" class="num text-right text-muted-foreground">{{ formatSize(b.sizeBytes) }}</span>
        <span role="cell">
          <Button variant="ghost" size="sm" :disabled="busy" @click="askRestore(b)">{{ t('settings.backup.restore') }}</Button>
        </span>
      </div>
    </div>

    <ConfirmDialog
      v-model:open="confirmOpen" destructive :title="confirmTitle" :description="t('settings.backup.restoreBody')"
      :confirm-label="t('settings.backup.restoreConfirm')" @confirm="onConfirm"
    />
  </div>
</template>
