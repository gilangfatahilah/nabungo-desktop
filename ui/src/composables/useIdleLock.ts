import { onScopeDispose, watch, type Ref } from 'vue'

const ACTIVITY = ['pointerdown', 'pointermove', 'keydown', 'wheel', 'touchstart'] as const

/** Panggil `onIdle` setelah `minutes` menit tanpa input pengguna, selama `enabled`. */
export function useIdleLock(minutes: Ref<number>, enabled: Ref<boolean>, onIdle: () => void): void {
  let timer: ReturnType<typeof setTimeout> | null = null

  function clear() {
    if (timer !== null) clearTimeout(timer)
    timer = null
  }

  function arm() {
    clear()
    if (enabled.value) timer = setTimeout(onIdle, minutes.value * 60_000)
  }

  // Pointermove bisa ratusan kali per detik; cukup geser tenggat bila timer aktif.
  const onActivity = () => {
    if (timer !== null) arm()
  }

  function listen(on: boolean) {
    for (const ev of ACTIVITY) {
      if (on) window.addEventListener(ev, onActivity, { passive: true })
      else window.removeEventListener(ev, onActivity)
    }
  }

  watch(
    [minutes, enabled],
    ([, on], prev) => {
      if (on !== prev?.[1]) listen(on)
      arm()
    },
    { immediate: true },
  )

  onScopeDispose(() => {
    clear()
    listen(false)
  })
}
