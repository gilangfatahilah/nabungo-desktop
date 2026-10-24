import { computed, type ComputedRef } from 'vue'
import { fontScaleRevision, rootFontScale } from '@/lib/fontScale'

/** Faktor skala ukuran teks yang reaktif: dihitung ulang setiap kali ukuran teks diterapkan. */
export function useRootFontScale(): ComputedRef<number> {
  return computed(() => {
    void fontScaleRevision.value
    return rootFontScale()
  })
}
