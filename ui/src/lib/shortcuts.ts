import type { RouteName } from '@/router'

export type ShortcutAction =
  | { kind: 'navigate'; route: RouteName }
  | { kind: 'newTxn' }
  | { kind: 'palette' }

interface Options {
  onAction: (a: ShortcutAction) => void
  now?: () => number
  sequenceTimeoutMs?: number
}

const GO: Record<string, RouteName> = {
  d: 'dashboard',
  t: 'transactions',
  b: 'budget',
  a: 'allocation',
  r: 'reports',
}

function isTyping(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null
  if (!el || !el.tagName) return false
  const tag = el.tagName.toLowerCase()
  return tag === 'input' || tag === 'textarea' || tag === 'select' || el.isContentEditable === true
}

// Handler keydown global (PRD §9). Urutan "G lalu huruf" kedaluwarsa setelah timeout.
export function createShortcutHandler(opts: Options): (e: KeyboardEvent) => void {
  const now = opts.now ?? (() => Date.now())
  const timeout = opts.sequenceTimeoutMs ?? 1000
  let gAt: number | null = null

  function fire(e: KeyboardEvent, a: ShortcutAction) {
    e.preventDefault()
    opts.onAction(a)
  }

  return (e) => {
    if (e.repeat || e.isComposing) return
    const key = e.key.toLowerCase()
    const mod = e.ctrlKey || e.metaKey

    if (mod && !e.altKey) {
      gAt = null
      if (key === 'k') return fire(e, { kind: 'palette' })
      if (key === ',') return fire(e, { kind: 'navigate', route: 'settings' })
      return
    }
    if (e.ctrlKey || e.metaKey || e.altKey || isTyping(e.target)) {
      gAt = null
      return
    }

    if (gAt !== null) {
      const fresh = now() - gAt <= timeout
      gAt = null
      const route = GO[key]
      if (fresh && route) return fire(e, { kind: 'navigate', route })
      if (fresh) return // tombol lain membatalkan urutan
    }

    if (key === 'g') {
      gAt = now()
      return
    }
    if (key === 'n') return fire(e, { kind: 'newTxn' })
  }
}
