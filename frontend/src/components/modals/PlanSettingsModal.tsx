import React, { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import i18n from '../../i18n'
import { Button } from '../shared/Button'
import {
  userLimitsGet,
  plansList,
  planChangeRequestCreate,
  type UserLimitsInfo,
  type PlanTierInfo,
} from '../../lib/api'

interface PlanSettingsPanelProps {
  onClose: () => void
}

function formatBytes(bytes: number): string {
  if (bytes < 0) return i18n.t('common.unlimited')
  if (bytes === 0) return '0 B'
  const gb = bytes / (1024 * 1024 * 1024)
  if (gb >= 1) return `${gb.toFixed(1)} GB`
  const mb = bytes / (1024 * 1024)
  return `${mb.toFixed(0)} MB`
}

function UsageBar({ used, max, label }: { used: number; max: number; label: string }) {
  const { t } = useTranslation()
  const pct = max <= 0 ? 0 : Math.min(100, Math.round((used / max) * 100))
  const color = pct >= 100 ? '#ef4444' : pct >= 90 ? '#f59e0b' : '#22c55e'
  return (
    <div style={{ marginBottom: 12 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 13, marginBottom: 4 }}>
        <span>{label}</span>
        <span style={{ color: 'var(--text-secondary)' }}>
          {max < 0 ? t('common.unlimited') : `${formatBytes(used)} / ${formatBytes(max)}`}
        </span>
      </div>
      {max > 0 && (
        <div style={{ height: 6, borderRadius: 3, background: 'var(--bg-active)', overflow: 'hidden' }}>
          <div style={{ height: '100%', width: `${pct}%`, background: color, borderRadius: 3, transition: 'width 0.3s' }} />
        </div>
      )}
    </div>
  )
}

export function PlanSettingsPanel({ onClose }: PlanSettingsPanelProps) {
  const { t } = useTranslation()
  const [limits, setLimits] = useState<UserLimitsInfo | null>(null)
  const [plans, setPlans] = useState<PlanTierInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedPlanId, setSelectedPlanId] = useState<string | null>(null)
  const [message, setMessage] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    const load = async () => {
      const [limitsResult, plansResult] = await Promise.all([userLimitsGet(), plansList()])
      if (cancelled) return
      if (limitsResult.success) setLimits(limitsResult.data)
      if (plansResult.success) setPlans(plansResult.data)
      setLoading(false)
    }
    void load()
    return () => { cancelled = true }
  }, [])

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!selectedPlanId) return
    setError(null)
    setSuccess(null)
    setSubmitting(true)
    try {
      const result = await planChangeRequestCreate(selectedPlanId, message.trim() || undefined)
      if (!result.success) {
        setError(result.error.message)
        return
      }
      setSuccess(t('planSettings.successRequest'))
      setSelectedPlanId(null)
      setMessage('')
    } catch {
      setError(t('planSettings.errRequest'))
    } finally {
      setSubmitting(false)
    }
  }

  if (loading) {
    return <div style={{ padding: 16, color: 'var(--text-secondary)' }}>{t('common.loadingShort')}</div>
  }

  return (
    <div className="modal-form">
      {limits && (
        <section style={{ marginBottom: 20 }}>
          <h4 style={{ marginBottom: 12 }}>{t('planSettings.currentPlan', { name: limits.plan.displayName })}</h4>

          <div style={{ marginBottom: 16 }}>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8, fontSize: 13 }}>
              <div><span style={{ color: 'var(--text-secondary)' }}>{t('planSettings.servers')} </span>{limits.usage.totalServers} / {limits.plan.limits.maxServers < 0 ? '∞' : limits.plan.limits.maxServers}</div>
              <div><span style={{ color: 'var(--text-secondary)' }}>{t('planSettings.members')} </span>{limits.usage.totalMembersAcrossServers} / {limits.plan.limits.maxMembersPerServer < 0 ? '∞' : limits.plan.limits.maxMembersPerServer}</div>
              <div><span style={{ color: 'var(--text-secondary)' }}>{t('planSettings.textChannels')} </span>{limits.usage.totalTextChannels} / {limits.plan.limits.maxChannelsTextPerServer < 0 ? '∞' : limits.plan.limits.maxChannelsTextPerServer}</div>
              <div><span style={{ color: 'var(--text-secondary)' }}>{t('planSettings.voiceChannels')} </span>{limits.usage.totalVoiceChannels} / {limits.plan.limits.maxChannelsVoicePerServer < 0 ? '∞' : limits.plan.limits.maxChannelsVoicePerServer}</div>
            </div>
          </div>

          {limits.plan.limits.maxStorageBytes > 0 && (
            <UsageBar used={limits.usage.storedBytes} max={limits.plan.limits.maxStorageBytes} label={t('planSettings.storage')} />
          )}
          {limits.plan.limits.maxTransferBytesMonthly > 0 && (
            <UsageBar used={limits.usage.transferBytesThisMonth} max={limits.plan.limits.maxTransferBytesMonthly} label={t('planSettings.transferMonthly')} />
          )}
        </section>
      )}

      <hr style={{ margin: '0 0 20px', border: 'none', borderTop: '1px solid var(--bg-active)' }} />

      <section>
        <h4 style={{ marginBottom: 12 }}>{t('planSettings.requestChange')}</h4>
        {plans.length > 0 ? (
          <form onSubmit={handleSubmit}>
            <div className="form-group">
              <label>{t('planSettings.desiredPlan')}</label>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {plans
                  .filter((p) => p.id !== limits?.plan.id)
                  .map((plan) => (
                    <label
                      key={plan.id}
                      style={{
                        display: 'flex',
                        alignItems: 'flex-start',
                        gap: 10,
                        padding: '10px 12px',
                        borderRadius: 6,
                        border: `1px solid ${selectedPlanId === plan.id ? 'var(--accent)' : 'var(--bg-active)'}`,
                        cursor: 'pointer',
                        background: selectedPlanId === plan.id ? 'var(--bg-active)' : 'transparent',
                      }}
                    >
                      <input
                        type="radio"
                        name="plan"
                        value={plan.id}
                        checked={selectedPlanId === plan.id}
                        onChange={() => setSelectedPlanId(plan.id)}
                        style={{ marginTop: 2 }}
                      />
                      <div>
                        <div style={{ fontWeight: 500 }}>{plan.displayName}</div>
                        {plan.description && (
                          <div style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 2 }}>{plan.description}</div>
                        )}
                      </div>
                    </label>
                  ))}
              </div>
            </div>

            <div className="form-group" style={{ marginTop: 12 }}>
              <label htmlFor="plan-message">{t('planSettings.messageLabel')}</label>
              <textarea
                id="plan-message"
                value={message}
                onChange={(e) => setMessage(e.target.value)}
                placeholder={t('planSettings.messagePlaceholder')}
                rows={3}
                style={{ resize: 'vertical' }}
                disabled={submitting}
              />
            </div>

            {error && <div className="modal-error">{error}</div>}
            {success && <div className="modal-success">{success}</div>}

            <div className="modal-form-actions">
              <Button type="button" variant="ghost" onClick={onClose} disabled={submitting}>
                {t('common.close')}
              </Button>
              <Button type="submit" variant="primary" disabled={submitting || !selectedPlanId}>
                {submitting ? t('planSettings.submitting') : t('planSettings.requestSubmit')}
              </Button>
            </div>
          </form>
        ) : (
          <p style={{ color: 'var(--text-secondary)', fontSize: 13 }}>{t('planSettings.noPlans')}</p>
        )}
      </section>
    </div>
  )
}
