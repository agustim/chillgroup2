import { useTranslation } from 'react-i18next'
import { SUPPORTED_LNGS } from '../../i18n'

interface LanguageSwitcherProps {
  className?: string
}

/**
 * Selector d'idioma. Canvia l'idioma actiu via i18next i el persisteix
 * a localStorage (clau 'lang', gestionada pel LanguageDetector).
 */
export function LanguageSwitcher({ className = '' }: LanguageSwitcherProps) {
  const { t, i18n } = useTranslation()
  const current = (i18n.resolvedLanguage ?? i18n.language).split('-')[0]

  return (
    <label className={`chillgroup-language-switcher ${className}`.trim()}>
      <span className="chillgroup-language-switcher__label">{t('language.label')}</span>
      <select
        className="chillgroup-language-switcher__select"
        value={current}
        onChange={(e) => i18n.changeLanguage(e.target.value)}
      >
        {SUPPORTED_LNGS.map((lng) => (
          <option key={lng} value={lng}>
            {t(`language.${lng}`)}
          </option>
        ))}
      </select>
    </label>
  )
}
