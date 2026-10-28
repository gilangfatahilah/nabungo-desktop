import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import * as ipc from '@/ipc'
import type { CaptureDto, InboxSummaryDto, UnparsedEmailDto } from '@/ipc'
import { fromCapture } from '@/lib/capture'
import { toDto, type TxnFormState } from '@/lib/txnForm'
import { useAccountsStore } from './accounts'
import { useCategoriesStore } from './categories'
import { useTransactionsStore } from './transactions'

// Inbox email capture (EML-03). Seperti store lain: setiap aksi tulis memuat ulang dari backend.
export const useInboxStore = defineStore('inbox', () => {
  const summary = ref<InboxSummaryDto>({ pending: 0, unparsed: 0, emailEnabled: false })
  const pending = ref<CaptureDto[]>([])
  const unparsed = ref<UnparsedEmailDto[]>([])
  const loaded = ref(false)
  /** K2: menu tidak tampil untuk user yang tidak memakai email capture. */
  const showNav = computed(() => summary.value.emailEnabled || summary.value.pending > 0 || summary.value.unparsed > 0)

  async function reloadSummary() {
    summary.value = await ipc.inboxSummary()
  }

  async function reload() {
    const [s, p, u] = await Promise.all([ipc.inboxSummary(), ipc.listPendingCaptures(), ipc.listUnparsedEmails()])
    summary.value = s
    pending.value = p
    unparsed.value = u
    loaded.value = true
  }

  /** Capture bisa sudah diproses di tempat lain (sync, jendela lain): daftar selalu dimuat ulang. */
  async function guarded<T>(action: () => Promise<T>): Promise<T> {
    try {
      return await action()
    } finally {
      await reload()
    }
  }

  /** Transaksi baru/terhubung memengaruhi saldo, daftar transaksi, dan kandidat duplikat. */
  async function posted() {
    await useTransactionsStore().refresh()
  }

  async function confirm(id: string, state: TxnFormState): Promise<string> {
    const saved = await guarded(() => ipc.confirmCapture(id, toDto(state)))
    await posted()
    await useTransactionsStore().notifyAlerts(saved.alerts)
    return saved.id
  }

  async function quickConfirm(c: CaptureDto): Promise<string> {
    const accounts = useAccountsStore()
    const categories = useCategoriesStore()
    const [defaults] = await Promise.all([ipc.txnFormDefaults(), accounts.ensureLoaded(), categories.ensureLoaded()])
    const state = fromCapture(c, {
      accounts: accounts.overview.accounts, categories: categories.list, feeCategoryId: defaults.feeCategoryId,
    })
    return confirm(c.id, state)
  }

  async function link(id: string, txnId: string) {
    await guarded(() => ipc.linkCapture(id, txnId))
    await posted()
  }

  async function ignore(id: string) {
    await guarded(() => ipc.ignoreCapture(id))
  }

  function ignoreUntil(date: string): Promise<string[]> {
    return guarded(() => ipc.ignoreCapturesUntil(date))
  }

  async function unignore(ids: string[]) {
    await guarded(() => ipc.unignoreCaptures(ids))
  }

  let listening: Promise<void> | null = null

  /** K10: sync (manual/terjadwal) selesai → badge, dan daftar bila sudah pernah dimuat. */
  function listen(): Promise<void> {
    listening ??= ipc
      .onEmailSync({
        done: () => void (loaded.value ? reload() : reloadSummary()).catch((e) => console.warn('gagal memuat ulang inbox', e)),
      })
      .then(() => {})
    return listening
  }

  return {
    summary, pending, unparsed, loaded, showNav,
    reloadSummary, reload, confirm, quickConfirm, link, ignore, ignoreUntil, unignore, listen,
  }
})
