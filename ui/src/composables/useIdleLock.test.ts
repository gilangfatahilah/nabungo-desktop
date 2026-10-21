import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { effectScope, nextTick, ref } from 'vue'
import { useIdleLock } from './useIdleLock'

const MIN = 60_000

describe('useIdleLock', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  function setup(minutes = 5, enabled = true) {
    const m = ref(minutes)
    const e = ref(enabled)
    const onIdle = vi.fn()
    const scope = effectScope()
    scope.run(() => useIdleLock(m, e, onIdle))
    return { m, e, onIdle, stop: () => scope.stop() }
  }

  it('memanggil onIdle setelah N menit tanpa aktivitas', () => {
    const { onIdle, stop } = setup(5)
    vi.advanceTimersByTime(5 * MIN - 1)
    expect(onIdle).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1)
    expect(onIdle).toHaveBeenCalledTimes(1)
    stop()
  })

  it('keydown me-reset timer', () => {
    const { onIdle, stop } = setup(1)
    vi.advanceTimersByTime(MIN - 1000)
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'a' }))
    vi.advanceTimersByTime(MIN - 1000)
    expect(onIdle).not.toHaveBeenCalled()
    vi.advanceTimersByTime(1000)
    expect(onIdle).toHaveBeenCalledTimes(1)
    stop()
  })

  it('tidak memanggil bila enabled=false', async () => {
    const { e, onIdle, stop } = setup(1, false)
    vi.advanceTimersByTime(10 * MIN)
    expect(onIdle).not.toHaveBeenCalled()
    e.value = true
    await nextTick()
    vi.advanceTimersByTime(MIN)
    expect(onIdle).toHaveBeenCalledTimes(1)
    e.value = false
    await nextTick()
    vi.advanceTimersByTime(10 * MIN)
    expect(onIdle).toHaveBeenCalledTimes(1)
    stop()
  })

  it('perubahan minutes berlaku', async () => {
    const { m, onIdle, stop } = setup(30)
    m.value = 1
    await nextTick()
    vi.advanceTimersByTime(MIN)
    expect(onIdle).toHaveBeenCalledTimes(1)
    stop()
  })

  it('berhenti saat scope dilepas', () => {
    const { onIdle, stop } = setup(1)
    stop()
    vi.advanceTimersByTime(10 * MIN)
    expect(onIdle).not.toHaveBeenCalled()
  })
})
