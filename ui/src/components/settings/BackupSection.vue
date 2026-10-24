<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import EmptyState from '@/components/EmptyState.vue'
import {
  AlertDialog, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  backupNow, inspectBackupSource, IpcFailure, listBackups, pickBackupFile, restoreBackup,
  type BackupDto, type BackupSourceDto,
} from '@/ipc'
import { formatBytes } from '@/lib/bytes'
import { formatTimestamp } from '@/lib/date'
import { errorMessage, type Translate } from '@/lib/errors'
import { withErrorToast } from '@/lib/notify'
import { reloadApp } from '@/lib/reload'
import { useSettingsStore } from '@/stores/settings'

const { t } = useI18n()
const tr = t as unknown as Translate
const settings = useSettingsStore()

const backups = ref<BackupDto[]>([])
const busy = ref(false)
const confirmOpen = ref(false)
// Tidak direset saat dialog ditutup, karena dialog bisa tertutup sebelum `confirm` diproses.
// `backup` = baris daftar (judul memuat waktunya); `null` = file pilihan user.
const pending = ref<{ source: BackupSourceDto; backup: BackupDto | null; needsKey: boolean } | null>(null)
/** Recovery key milik backup terenkripsi instalasi lain. */
const recoveryKey = ref('')
const keyError = ref<string | null>(null)

const when = (b: BackupDto) => formatTimestamp(b.createdAt, settings.locale)
const confirmTitle = computed(() =>
  pending.value?.backup
    ? t('settings.backup.restoreTitle', { time: when(pending.value.backup) })
    : t('settings.backup.restoreFileTitle'),
)
const canConfirm = computed(() => !busy.value && !(pending.value?.needsKey && recoveryKey.value.trim() === ''))

const formatSize = (bytes: number) => formatBytes(bytes, settings.locale)

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

/** Periksa sumber dulu: backup terenkripsi dengan key lain butuh recovery key-nya. */
async function ask(source: BackupSourceDto, backup: BackupDto | null) {
  const { needsKey } = await inspectBackupSource(source)
  pending.value = { source, backup, needsKey }
  recoveryKey.value = ''
  keyError.value = null
  confirmOpen.value = true
}

function askRestore(b: BackupDto) {
  void run(() => ask({ kind: 'named', fileName: b.fileName }, b))
}

function askRestoreFile() {
  void run(async () => {
    const path = await pickBackupFile()
    if (path) await ask({ kind: 'path', path }, null)
  })
}

const KEY_ERRORS = new Set(['invalid_recovery_key', 'backup_key_required'])

async function onConfirm() {
  const target = pending.value
  if (!target) return
  busy.value = true
  keyError.value = null
  try {
    await restoreBackup(target.source, target.needsKey ? recoveryKey.value : null)
    confirmOpen.value = false
    toast.success(t('settings.backup.restored'))
    reloadApp()
  } catch (e) {
    if (target.needsKey && e instanceof IpcFailure && KEY_ERRORS.has(e.error.code)) keyError.value = errorMessage(e, tr)
    else {
      confirmOpen.value = false
      toast.error(errorMessage(e, tr))
    }
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <p class="max-w-prose text-muted-foreground">{{ t('settings.backup.intro') }}</p>
    <div class="flex flex-wrap gap-2">
      <Button :disabled="busy" @click="onBackupNow">{{ t('settings.backup.backupNow') }}</Button>
      <Button variant="outline" :disabled="busy" @click="askRestoreFile">{{ t('settings.backup.restoreFromFile') }}</Button>
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

    <AlertDialog v-model:open="confirmOpen">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{{ confirmTitle }}</AlertDialogTitle>
          <AlertDialogDescription>{{ t('settings.backup.restoreBody') }}</AlertDialogDescription>
        </AlertDialogHeader>
        <form v-if="pending?.needsKey" class="flex flex-col gap-2" novalidate @submit.prevent="canConfirm && onConfirm()">
          <Label for="restore-recovery-key">{{ t('settings.backup.recoveryKey') }}</Label>
          <Input
            id="restore-recovery-key" v-model="recoveryKey" autocomplete="off" spellcheck="false" class="font-mono"
            :aria-invalid="keyError ? 'true' : undefined"
            :aria-describedby="keyError ? 'restore-recovery-key-error' : 'restore-recovery-key-help'"
          />
          <p v-if="keyError" id="restore-recovery-key-error" class="text-meta text-destructive" role="alert">{{ keyError }}</p>
          <p v-else id="restore-recovery-key-help" class="text-meta text-muted-foreground">{{ t('settings.backup.recoveryKeyHelp') }}</p>
        </form>
        <AlertDialogFooter>
          <AlertDialogCancel>{{ t('actions.cancel') }}</AlertDialogCancel>
          <!-- Bukan AlertDialogAction: dialog tetap terbuka bila recovery key salah. -->
          <Button variant="destructive" :disabled="!canConfirm" @click="onConfirm">{{ t('settings.backup.restoreConfirm') }}</Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
