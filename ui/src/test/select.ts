import { flushPromises } from '@vue/test-utils'
import { nextTick } from 'vue'

// Helper test untuk pemilih berbasis IconSelect (AccountSelect/CategorySelect): pemicu berupa tombol
// combobox yang membuka popover berisi daftar opsi (di-teleport ke body).

async function settle() {
  await flushPromises()
  await nextTick()
  await flushPromises()
}

/** Label opsi terpilih (atau placeholder) yang tampil di tombol pemicu. */
export function selectedLabel(trigger: Element): string {
  return trigger.textContent?.trim() ?? ''
}

/** Buka popover lalu klik opsi dengan label persis `label`, atau yang memuatnya (`Parent › Child`). */
export async function pickOption(trigger: HTMLElement, label: string) {
  trigger.click()
  await settle()
  const options = [...document.body.querySelectorAll<HTMLElement>('[role="option"]')]
  const opt = options.find((o) => o.textContent?.trim() === label) ?? options.find((o) => o.textContent?.includes(label))
  if (!opt) throw new Error(`opsi "${label}" tidak ada: ${options.map((o) => o.textContent?.trim()).join(', ')}`)
  opt.click()
  await settle()
}
