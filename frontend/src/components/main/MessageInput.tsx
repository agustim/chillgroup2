import React from 'react'
import { EncryptionType } from '../../types'
import { Button } from '../shared/Button'

interface MessageInputProps {
  value: string
  onChange: (value: string) => void
  onKeyDown?: (e: React.KeyboardEvent) => void
  onSubmit?: () => void
  placeholder?: string
  encryptionType: EncryptionType
}

export function MessageInput({
  value,
  onChange,
  onKeyDown,
  onSubmit,
  placeholder,
  encryptionType,
}: MessageInputProps) {
  const handleSubmit = () => {
    if (onSubmit && value.trim()) {
      onSubmit()
    }
  }

  const cryptoIndicator = {
    none: null,
    symmetric: '🔑',
    asymmetric: '🔒',
  }[encryptionType]

  return (
    <div className="message-input">
      <div className="message-input-content">
        <input
          type="text"
          className="message-input-field"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={onKeyDown}
          onSubmit={handleSubmit}
          placeholder={placeholder || 'Escriu un missatge...'}
          autoComplete="off"
        />
        {cryptoIndicator && (
          <span className="input-crypto-indicator" title={`Encriptació ${encryptionType}`}>
            {cryptoIndicator}
          </span>
        )}
      </div>
      <Button
        variant="primary"
        size="sm"
        onClick={handleSubmit}
        disabled={!value.trim()}
        className="send-button"
      >
        📤
      </Button>
    </div>
  )
}