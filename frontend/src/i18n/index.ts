import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'

import ca from './locales/ca/translation.json'
import en from './locales/en/translation.json'

export const SUPPORTED_LNGS = ['ca', 'en'] as const
export type SupportedLng = (typeof SUPPORTED_LNGS)[number]

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      ca: { translation: ca },
      en: { translation: en },
    },
    fallbackLng: 'ca',
    supportedLngs: SUPPORTED_LNGS,
    interpolation: {
      escapeValue: false, // React ja escapa
    },
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
      lookupLocalStorage: 'lang',
    },
  })

export default i18n
