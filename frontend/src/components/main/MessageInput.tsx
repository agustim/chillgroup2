import React, { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
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
  replyTo?: { messageId: string; senderUsername: string; text: string } | null
  onClearReplyTo?: () => void
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
  replyTo,
  onClearReplyTo,
}: MessageInputProps) {
  const { t } = useTranslation()
  const fileInputRef = React.useRef<HTMLInputElement>(null)
  const textInputRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    if (focusKey !== undefined) {
      textInputRef.current?.focus()
    }
  }, [focusKey])

  // Auto-resize textarea as content grows
  useEffect(() => {
    const el = textInputRef.current
    if (!el) return
    el.style.height = 'auto'
    el.style.height = `${el.scrollHeight}px`
  }, [value])

  const handleSubmit = () => {
    if (isBusy) return
    if (onSubmit && (value.trim() || pendingAttachments.length > 0)) {
      onSubmit()
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSubmit()
      return
    }
    onKeyDown?.(e)
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
      {replyTo && (
        <div className="reply-to-bar">
          <span className="reply-to-label">Responent a <strong>{replyTo.senderUsername}</strong>{replyTo.text ? `: ${replyTo.text.slice(0, 80)}${replyTo.text.length > 80 ? '…' : ''}` : ''}</span>
          <button type="button" className="reply-to-clear" onClick={onClearReplyTo} aria-label="Cancel·lar resposta">×</button>
        </div>
      )}
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
          <textarea
            ref={textInputRef}
            className="message-input-field"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={placeholder || 'Escriu un missatge...'}
            autoComplete="off"
            disabled={isBusy}
            rows={1}
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
