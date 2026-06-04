import React from 'react'

interface PanelTabProps {
  icon: string
  label: string
  isActive: boolean
  onClick: () => void
  onClose: () => void
  closeTitle?: string
  unreadCount?: number
}

export function PanelTab({
  icon,
  label,
  isActive,
  onClick,
  onClose,
  closeTitle = 'Tancar pestanya',
  unreadCount,
}: PanelTabProps) {
  if (!isActive) return null
  return (
    <div className="main-content-tab active" onClick={onClick}>
      <span>{icon}</span>
      <span>{label}</span>
      {(unreadCount ?? 0) > 0 && (
        <span className="channel-unread-badge">{unreadCount}</span>
      )}
      <button
        type="button"
        className="main-content-tab-close"
        onClick={(event) => {
          event.stopPropagation()
          onClose()
        }}
        title={closeTitle}
      >
        ✕
      </button>
    </div>
  )
}
