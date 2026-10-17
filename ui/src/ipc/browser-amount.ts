// Port `crates/core/src/amount.rs` untuk mode browser. Aturan harus tetap sama dengan Rust;
// tabel uji di browser-amount.test.ts disalin dari test Rust.
import type { ErrorCode } from './bindings'

const MAX = 1_000_000_000_000n

type Parsed = { ok: number } | { err: ErrorCode }

class Fail {
  readonly code: ErrorCode
  constructor(code: ErrorCode) {
    this.code = code
  }
}

const isDigit = (c: string | undefined) => c !== undefined && c >= '0' && c <= '9'

/** `["-"] suku (("+"|"-") suku)*`, suku = `angka [rb|jt]`; `.` ribuan, `,` desimal. */
export function parseAmountBrowser(input: string): Parsed {
  const s = input.replace(/\s/g, '').toLowerCase()
  if (!s) return { err: 'amount_empty' }
  try {
    let pos = s[0] === '-' ? 1 : 0
    let sign = pos ? -1n : 1n
    let total = 0n
    for (;;) {
      const [term, next] = parseTerm(s, pos)
      pos = next
      if (term > MAX) throw new Fail('amount_too_large')
      total += sign * term
      if (total > MAX || -total > MAX) throw new Fail('amount_too_large')
      const c = s[pos]
      if (c === undefined) break
      if (c === '+') sign = 1n
      else if (c === '-') sign = -1n
      else throw new Fail('amount_invalid')
      pos++
    }
    return { ok: Number(total) }
  } catch (e) {
    if (e instanceof Fail) return { err: e.code }
    throw e
  }
}

function parseTerm(s: string, start: number): [bigint, number] {
  let pos = start
  const digits = () => {
    const b = pos
    while (isDigit(s[pos])) pos++
    return s.slice(b, pos)
  }
  let int = digits()
  if (!int) throw new Fail('amount_invalid')
  while (s[pos] === '.') {
    pos++
    const g = digits()
    if (g.length !== 3) throw new Fail('amount_invalid')
    int += g
  }
  let frac = ''
  if (s[pos] === ',') {
    pos++
    frac = digits()
    if (!frac) throw new Fail('amount_invalid')
  }
  const suffix = s.slice(pos, pos + 2)
  const mult = suffix === 'rb' ? 1_000n : suffix === 'jt' ? 1_000_000n : 1n
  if (mult > 1n) pos += 2
  if (s[pos] === ',' || s[pos] === '.' || isDigit(s[pos])) throw new Fail('amount_invalid')
  let value = BigInt(int) * mult
  frac = frac.replace(/0+$/, '')
  if (frac) {
    if (frac.length > 6) throw new Fail('amount_fractional')
    const scaled = BigInt(frac) * mult
    const div = 10n ** BigInt(frac.length)
    if (scaled % div !== 0n) throw new Fail('amount_fractional')
    value += scaled / div
  }
  return [value, pos]
}
