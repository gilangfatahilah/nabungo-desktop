import { toast } from 'vue-sonner'
import { i18n } from '@/i18n'
import type { BudgetAlertDto } from '@/ipc'

// Toast peringatan budget (BUD-04) dari alert yang dikembalikan backend saat transaksi disimpan.
export function showBudgetAlerts(alerts: BudgetAlertDto[], names: Map<string, string>): void {
  const { t } = i18n.global
  for (const a of alerts) {
    const name = names.get(a.categoryId) ?? ''
    if (a.level === 'warning') toast.warning(t('budget.alertWarning', { name, percent: a.percent ?? 0 }))
    else if (a.percent === null) toast.error(t('budget.alertExceededNoPercent', { name }))
    else toast.error(t('budget.alertExceeded', { name, percent: a.percent }))
  }
}
