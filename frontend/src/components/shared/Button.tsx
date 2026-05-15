import React from 'react'

export type ButtonVariant = 'primary' | 'secondary' | 'danger' | 'ghost'
export type ButtonSize = 'sm' | 'md' | 'lg'

interface ButtonProps {
  children: React.ReactNode
  variant?: ButtonVariant
  size?: ButtonSize
  disabled?: boolean
  onClick?: () => void
  className?: string
  type?: 'button' | 'submit' | 'reset'
}

export function Button({
  children,
  variant = 'primary',
  size = 'md',
  disabled = false,
  onClick,
  className = '',
  type = 'button',
}: ButtonProps) {
  const baseClasses = 'chillgroup-button'
  const variantClasses = `chillgroup-button--${variant}`
  const sizeClasses = `chillgroup-button--${size}`
  const disabledClasses = disabled ? 'chillgroup-button--disabled' : ''
  const classes = `${baseClasses} ${variantClasses} ${sizeClasses} ${disabledClasses} ${className}`.trim()

  return (
    <button type={type} className={classes} disabled={disabled} onClick={onClick}>
      {children}
    </button>
  )
}