import { createI18n } from 'vue-i18n'
import en from './en'
import id from './id'

export type MessageSchema = typeof id

export const i18n = createI18n({
  legacy: false,
  locale: 'id',
  fallbackLocale: 'en',
  messages: { id, en },
})
