import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { i18n } from '@/i18n'
import EncryptionFields, { type EncryptionInput } from './EncryptionFields.vue'

function mountFields() {
  const w = mount(EncryptionFields, {
    props: {
      modelValue: { password: '', confirm: '', acknowledged: false } satisfies EncryptionInput,
      'onUpdate:modelValue': (v: EncryptionInput) => w.setProps({ modelValue: v }),
    },
    global: { plugins: [i18n] },
  })
  return w
}
const lastValid = (w: ReturnType<typeof mountFields>) => w.emitted('valid')!.at(-1)![0]

describe('EncryptionFields', () => {
  it('tidak valid sampai peringatan dicentang', async () => {
    const w = mountFields()
    expect(lastValid(w)).toBe(false)
    await w.find('#enc-password').setValue('rahasia-123')
    await w.find('#enc-confirm').setValue('rahasia-123')
    expect(lastValid(w)).toBe(false)
    await w.find('#enc-ack').trigger('click')
    expect(lastValid(w)).toBe(true)
  })

  it('konfirmasi berbeda → tidak valid', async () => {
    const w = mountFields()
    await w.find('#enc-ack').trigger('click')
    await w.find('#enc-password').setValue('rahasia-123')
    await w.find('#enc-confirm').setValue('rahasia-124')
    expect(lastValid(w)).toBe(false)
    await w.find('#enc-confirm').setValue('rahasia-123')
    expect(lastValid(w)).toBe(true)
  })

  it('password < 8 karakter → tidak valid', async () => {
    const w = mountFields()
    await w.find('#enc-ack').trigger('click')
    await w.find('#enc-password').setValue('pendek')
    await w.find('#enc-confirm').setValue('pendek')
    expect(lastValid(w)).toBe(false)
  })
})
