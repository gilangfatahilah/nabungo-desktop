import { formatCompact, formatIdr, moneyParts } from '@/lib/money'
import { MASK, MASK_SHORT } from '@/lib/privacy'
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'

// Format nominal untuk tampilan baca (kartu, grafik) yang menghormati mode sembunyikan nominal.
// Form & dialog aksi tetap memakai `lib/money.ts` langsung agar isian selalu terlihat.
export function useMoneyFormat() {
  const settings = useSettingsStore()
  const ui = useUiStore()
  return {
    /** "Rp 1.250.000" atau "Rp ••••••••". */
    money: (v: number) =>
      ui.amountsHidden ? `${moneyParts(0, settings.locale).currency} ${MASK}` : formatIdr(v, settings.locale),
    /** Angka bertanda tanpa mata uang ("−1.250.000") atau "••••••••" (legenda, badge). */
    num: (v: number) => {
      if (ui.amountsHidden) return MASK
      const p = moneyParts(v, settings.locale)
      return p.sign + p.number
    },
    /** "1,3 jt" atau "•••" (total donut, label bar). */
    compact: (v: number) => (ui.amountsHidden ? MASK_SHORT : formatCompact(v, settings.locale)),
    /** Label sumbu grafik (nilai tick bisa pecahan). */
    axis: (v: number) => (ui.amountsHidden ? MASK_SHORT : formatCompact(Math.round(v), settings.locale)),
  }
}
