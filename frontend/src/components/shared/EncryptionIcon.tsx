import React from 'react'
import { EncryptionType } from '../../types'

interface EncryptionIconProps {
  type: EncryptionType
}

export function EncryptionIcon({ type }: EncryptionIconProps) {
  if (type === 'none') return null

  const icons = {
    symmetric: '🔑',
    asymmetric: '🔒',
  }

  const titles = {
    symmetric: 'Encriptació simètrica',
    asymmetric: 'Encriptació asimètrica (E2EE)',
  }

  return (
    <span
      className={`encryption-icon encryption-${type}`}
      title={titles[type]}
      aria-label={titles[type]}
    >
      {icons[type]}
    </span>
  )
}