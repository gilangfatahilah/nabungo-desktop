import { describe, expect, it } from 'vitest'
import { parseAmountBrowser } from './browser-amount'

// Tabel sama dengan crates/core/src/amount.rs.
describe('parseAmountBrowser (paritas Rust)', () => {
  it.each([
    ['25000', 25_000], ['25.000', 25_000], ['1.250.000', 1_250_000], ['25rb', 25_000], ['25 RB', 25_000],
    ['1,5jt', 1_500_000], ['1,25rb', 1_250], ['2jt', 2_000_000], ['25000+12500', 37_500],
    ['50rb - 12.500', 37_500], ['-5000', -5_000], ['1jt+250rb+3.000', 1_253_000], ['0', 0],
    ['1.000.000jt', 1_000_000_000_000],
  ])('%s → %d', (input, want) => {
    expect(parseAmountBrowser(input)).toEqual({ ok: want })
  })

  it.each([
    ['', 'amount_empty'], ['   ', 'amount_empty'], ['1.5', 'amount_invalid'], ['25.00', 'amount_invalid'],
    ['1.2345', 'amount_invalid'], ['abc', 'amount_invalid'], ['25++1', 'amount_invalid'], ['25+', 'amount_invalid'],
    ['rb', 'amount_invalid'], ['1,5,0jt', 'amount_invalid'], ['25.000,50', 'amount_fractional'],
    ['10,5', 'amount_fractional'], ['1,2345rb', 'amount_fractional'], ['1000001jt', 'amount_too_large'],
    ['999999999999999999999999', 'amount_too_large'], ['1.000.000jt+1', 'amount_too_large'],
  ])('%s → %s', (input, code) => {
    expect(parseAmountBrowser(input)).toEqual({ err: code })
  })
})
