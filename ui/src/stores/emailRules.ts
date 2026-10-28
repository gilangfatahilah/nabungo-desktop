import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as ipc from '@/ipc'
import type { EmailRuleDto, EmailRuleInputDto, RuleSavedDto } from '@/ipc'
import { useInboxStore } from './inbox'

// Rule email capture (EML-04). Seperti store lain: setiap aksi tulis memuat ulang dari backend.
// Uji & saran rule dipanggil langsung lewat `ipc` dari sheet (tidak menulis).
export const useEmailRulesStore = defineStore('emailRules', () => {
  const rules = ref<EmailRuleDto[]>([])
  const loaded = ref(false)

  async function reload() {
    rules.value = await ipc.listEmailRules()
    loaded.value = true
  }

  /** Re-parse memindahkan email gagal dibaca ke pending: daftar inbox ikut dimuat ulang. */
  async function written(saved: RuleSavedDto): Promise<RuleSavedDto> {
    await reload()
    if (saved.reparsed.emails > 0) await useInboxStore().refresh()
    return saved
  }

  async function save(id: string | null, input: EmailRuleInputDto): Promise<RuleSavedDto> {
    return written(await (id === null ? ipc.createEmailRule(input) : ipc.updateEmailRule(id, input)))
  }

  async function setEnabled(id: string, on: boolean): Promise<RuleSavedDto> {
    return written(await ipc.setEmailRuleEnabled(id, on))
  }

  async function remove(id: string) {
    await ipc.deleteEmailRule(id)
    await reload()
  }

  async function restore(id: string): Promise<RuleSavedDto> {
    return written(await ipc.restoreEmailRule(id))
  }

  return { rules, loaded, reload, save, setEnabled, remove, restore }
})
