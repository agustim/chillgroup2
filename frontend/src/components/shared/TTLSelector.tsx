import { useState } from 'react'

const TTL_OPTIONS: { label: string; value: number | null }[] = [
  { label: 'Sense expiració', value: null },
  { label: '1 hora', value: 3600 },
  { label: '1 dia', value: 86400 },
  { label: '7 dies', value: 604800 },
  { label: '30 dies', value: 2592000 },
]

export function formatTTL(seconds: number | null | undefined): string {
  if (!seconds) return '∞'
  if (seconds < 3600) return `${Math.round(seconds / 60)}min`
  if (seconds < 86400) return `${Math.round(seconds / 3600)}h`
  if (seconds < 604800) return `${Math.round(seconds / 86400)}d`
  if (seconds < 2592000) return `${Math.round(seconds / 604800)}set`
  return `${Math.round(seconds / 2592000)}mes`
}

interface TTLSelectorProps {
  value: number | null
  onChange: (ttl: number | null) => void
  disabled?: boolean
}

export function TTLSelector({ value, onChange, disabled = false }: TTLSelectorProps) {
  const [customTTL, setCustomTTL] = useState('')

  const applyCustom = () => {
    const secs = parseInt(customTTL, 10)
    if (secs > 0) {
      onChange(secs)
      setCustomTTL('')
    }
  }

  return (
    <div className="ttl-selector">
      <div className="ttl-selector__presets">
        {TTL_OPTIONS.map((opt) => (
          <button
            key={String(opt.value)}
            type="button"
            className={`chillgroup-button chillgroup-button--sm ${value === opt.value ? 'chillgroup-button--primary' : 'chillgroup-button--ghost'}`}
            onClick={() => onChange(opt.value)}
            disabled={disabled}
          >
            {opt.label}
          </button>
        ))}
      </div>
      <div className="ttl-selector__custom">
        <input
          type="number"
          min="1"
          placeholder="Segons personalitzats"
          value={customTTL}
          onChange={(e) => setCustomTTL(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); applyCustom() } }}
          disabled={disabled}
          className="chillgroup-input chillgroup-input--sm"
        />
        <button
          type="button"
          className="chillgroup-button chillgroup-button--secondary chillgroup-button--sm"
          onClick={applyCustom}
          disabled={disabled || !customTTL || parseInt(customTTL, 10) <= 0}
        >
          Aplicar
        </button>
      </div>
    </div>
  )
}
