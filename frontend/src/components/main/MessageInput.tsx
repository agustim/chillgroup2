import React, { useEffect, useRef } from 'react'
import { EncryptionType } from '../../types'
import { Button } from '../shared/Button'

interface PendingAttachmentItem {
  id: string
  name: string
  size: number
}

interface MessageInputProps {
  value: string
  onChange: (value: string) => void
  onKeyDown?: (e: React.KeyboardEvent) => void
  onSubmit?: () => void
  onAddAttachments?: (files: FileList | null) => void
  onRemoveAttachment?: (attachmentId: string) => void
  pendingAttachments?: PendingAttachmentItem[]
  placeholder?: string
  encryptionType: EncryptionType
  isBusy?: boolean
  focusKey?: string
}

function formatFileSize(size: number): string {
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${(size / (1024 * 1024)).toFixed(1)} MB`
}

export function MessageInput({
  value,
  onChange,
  onKeyDown,
  onSubmit,
  onAddAttachments,
  onRemoveAttachment,
  pendingAttachments = [],
  placeholder,
  encryptionType,
  isBusy = false,
  focusKey,
}: MessageInputProps) {
  const fileInputRef = React.useRef<HTMLInputElement>(null)
  const textInputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (focusKey !== undefined) {
      textInputRef.current?.focus()
    }
  }, [focusKey])

  const handleSubmit = () => {
    if (isBusy) return
    if (onSubmit && (value.trim() || pendingAttachments.length > 0)) {
      onSubmit()
    }
  }

  const handlePickFiles = () => {
    if (isBusy) return
    fileInputRef.current?.click()
  }

  const handleFilesChanged = (e: React.ChangeEvent<HTMLInputElement>) => {
    onAddAttachments?.(e.target.files)
    e.target.value = ''
  }

  const cryptoIndicator = {
    none: null,
    symmetric: '🔑',
    asymmetric: '🔒',
  }[encryptionType]

  return (
    <div className="message-input">
      {pendingAttachments.length > 0 && (
        <div className="message-attachments-preview">
          {pendingAttachments.map((attachment) => (
            <div key={attachment.id} className="message-attachment-chip">
              <span className="message-attachment-chip-name" title={attachment.name}>
                {attachment.name}
              </span>
              <span className="message-attachment-chip-size">{formatFileSize(attachment.size)}</span>
              <button
                type="button"
                className="message-attachment-chip-remove"
                onClick={() => onRemoveAttachment?.(attachment.id)}
                disabled={isBusy}
                aria-label={`Eliminar ${attachment.name}`}
              >
                ×
              </button>
            </div>
          ))}
        </div>
      )}

      <div className="message-input-actions">
        <Button
          variant="secondary"
          size="sm"
          onClick={handlePickFiles}
          disabled={isBusy}
          className="attach-button"
        >
          📎
        </Button>
        <input
          ref={fileInputRef}
          type="file"
          multiple
          className="message-file-input"
          onChange={handleFilesChanged}
        />

      <div className="message-input-content">
        <input
          ref={textInputRef}
          type="text"
          className="message-input-field"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={onKeyDown}
          onSubmit={handleSubmit}
          placeholder={placeholder || 'Escriu un missatge...'}
          autoComplete="off"
          disabled={isBusy}
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
        disabled={isBusy || (!value.trim() && pendingAttachments.length === 0)}
        className="send-button"
      >
        📤
      </Button>
      </div>
    </div>
  )
}