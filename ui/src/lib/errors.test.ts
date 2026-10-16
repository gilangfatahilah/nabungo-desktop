import { afterEach, describe, expect, it } from 'vitest'
import { i18n } from '@/i18n'
import { IpcFailure, type ErrorCode } from '@/ipc'
import { errorMessage } from './errors'

const t = i18n.global.t

describe('errorMessage', () => {
  afterEach(() => {
    i18n.global.locale.value = 'id'
  })

  it('menerjemahkan kode domain', () => {
    const e = new IpcFailure({ code: 'txn_transfer_same_account', entity: null })
    expect(errorMessage(e, t)).toBe('Akun asal dan tujuan transfer harus berbeda')
  })

  it('menyisipkan entity', () => {
    const e = new IpcFailure({ code: 'not_found', entity: 'Akun' })
    expect(errorMessage(e, t)).toBe('Akun tidak ditemukan')
  })

  it('error lain jatuh ke internal', () => {
    expect(errorMessage(new Error('x'), t)).toBe('Terjadi kesalahan. Detail ada di log aplikasi.')
  })

  it('IpcFailure dengan code hilang/tak dikenal jatuh ke internal', () => {
    const bad = new IpcFailure('boom' as unknown as { code: ErrorCode; entity: string | null })
    const unknown = new IpcFailure({ code: 'xyz' as ErrorCode, entity: null })
    expect(errorMessage(bad, t)).toBe('Terjadi kesalahan. Detail ada di log aplikasi.')
    expect(errorMessage(unknown, t)).toBe('Terjadi kesalahan. Detail ada di log aplikasi.')
  })

  it('mengikuti locale', () => {
    i18n.global.locale.value = 'en'
    const e = new IpcFailure({ code: 'txn_transfer_same_account', entity: null })
    expect(errorMessage(e, t)).toBe('Transfer source and destination accounts must differ')
  })
})
