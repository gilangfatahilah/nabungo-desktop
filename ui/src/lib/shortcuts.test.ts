import { describe, expect, it } from 'vitest'
import { createShortcutHandler, type ShortcutAction } from './shortcuts'

function setup() {
  const actions: ShortcutAction[] = []
  let t = 0
  const handler = createShortcutHandler({ onAction: (a) => actions.push(a), now: () => t })
  function press(target: Element, key: string, init: KeyboardEventInit = {}) {
    const e = new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init })
    Object.defineProperty(e, 'target', { value: target })
    handler(e)
    return e
  }
  return { actions, press, setNow: (v: number) => (t = v) }
}

describe('shortcuts', () => {
  it('n di body membuka transaksi baru', () => {
    const { actions, press } = setup()
    const e = press(document.body, 'n')
    expect(actions).toEqual([{ kind: 'newTxn' }])
    expect(e.defaultPrevented).toBe(true)
  })

  it('g lalu t dalam 500 ms menavigasi', () => {
    const { actions, press, setNow } = setup()
    press(document.body, 'g')
    setNow(500)
    press(document.body, 't')
    expect(actions).toEqual([{ kind: 'navigate', route: 'transactions' }])
  })

  it('g lalu t setelah 1500 ms tidak melakukan apa pun', () => {
    const { actions, press, setNow } = setup()
    press(document.body, 'g')
    setNow(1500)
    press(document.body, 't')
    expect(actions).toEqual([])
  })

  it('g x d: x membatalkan urutan, d sendiri tidak memicu', () => {
    const { actions, press } = setup()
    press(document.body, 'g')
    press(document.body, 'x')
    press(document.body, 'd')
    expect(actions).toEqual([])
  })

  it.each([
    ['dashboard', 'd'],
    ['transactions', 't'],
    ['budget', 'b'],
    ['allocation', 'a'],
    ['reports', 'r'],
  ])('g lalu huruf ke %s', (route, k) => {
    const { actions, press } = setup()
    press(document.body, 'g')
    press(document.body, k)
    expect(actions).toEqual([{ kind: 'navigate', route }])
  })

  it('diabaikan di input, textarea, contenteditable', () => {
    const { actions, press } = setup()
    const div = document.createElement('div')
    div.setAttribute('contenteditable', 'true')
    // jsdom tidak mengisi isContentEditable
    Object.defineProperty(div, 'isContentEditable', { value: true })
    for (const el of [document.createElement('input'), document.createElement('textarea'), div]) {
      press(el, 'n')
      press(el, 'g')
      press(el, 'd')
    }
    expect(actions).toEqual([])
  })

  it('Ctrl+K dari input membuka palette; Ctrl+, ke pengaturan; Ctrl+N diabaikan', () => {
    const { actions, press } = setup()
    press(document.createElement('input'), 'k', { ctrlKey: true })
    press(document.body, ',', { ctrlKey: true })
    press(document.body, 'n', { ctrlKey: true })
    expect(actions).toEqual([{ kind: 'palette' }, { kind: 'navigate', route: 'settings' }])
  })

  it('Ctrl+B tidak di-preventDefault', () => {
    const { actions, press } = setup()
    const e = press(document.body, 'b', { ctrlKey: true })
    expect(e.defaultPrevented).toBe(false)
    expect(actions).toEqual([])
  })

  it('Meta+K membuka palette', () => {
    const { actions, press } = setup()
    press(document.body, 'k', { metaKey: true })
    expect(actions).toEqual([{ kind: 'palette' }])
  })

  it('Alt+huruf diabaikan', () => {
    const { actions, press } = setup()
    const e = press(document.body, 'n', { altKey: true })
    expect(actions).toEqual([])
    expect(e.defaultPrevented).toBe(false)
  })

  it('event repeat dan composing diabaikan', () => {
    const { actions, press } = setup()
    press(document.body, 'n', { repeat: true })
    press(document.body, 'n', { isComposing: true })
    expect(actions).toEqual([])
  })

  it('Ctrl+= / Ctrl++ / Ctrl+- / Ctrl+0 mengubah ukuran teks', () => {
    const { actions, press } = setup()
    press(document.body, '=', { ctrlKey: true })
    press(document.body, '+', { ctrlKey: true, shiftKey: true })
    press(document.body, '-', { ctrlKey: true })
    press(document.body, '0', { metaKey: true })
    expect(actions).toEqual([
      { kind: 'fontScale', step: 1 },
      { kind: 'fontScale', step: 1 },
      { kind: 'fontScale', step: -1 },
      { kind: 'fontScale', step: 0 },
    ])
  })

  it('shortcut ukuran teks tetap jalan di input dan tidak mengetik', () => {
    const { actions, press } = setup()
    const e = press(document.createElement('input'), '-', { ctrlKey: true })
    expect(actions).toEqual([{ kind: 'fontScale', step: -1 }])
    expect(e.defaultPrevented).toBe(true)
  })

  it('= tanpa modifier tidak mengubah ukuran teks', () => {
    const { actions, press } = setup()
    press(document.body, '=')
    press(document.body, '-')
    expect(actions).toEqual([])
  })

  it('Ctrl+Shift+H menyembunyikan/menampilkan nominal, juga dari dalam input', () => {
    const { actions, press } = setup()
    const e = press(document.createElement('input'), 'H', { ctrlKey: true, shiftKey: true })
    expect(actions).toEqual([{ kind: 'toggleAmounts' }])
    expect(e.defaultPrevented).toBe(true)
  })

  it('Ctrl+H tanpa Shift tidak melakukan apa pun', () => {
    const { actions, press } = setup()
    press(document.body, 'h', { ctrlKey: true })
    expect(actions).toEqual([])
  })
})
