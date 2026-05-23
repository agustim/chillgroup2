import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor, fireEvent } from '@testing-library/react'
import '@testing-library/jest-dom'

import { DeviceKeysModal } from './DeviceKeysModal'

vi.mock('../../lib/device-keys', () => ({
  deleteDeviceKeypair: vi.fn(),
  deleteSymmetricChannelKey: vi.fn(),
  exportDeviceKeypair: vi.fn(),
  exportAsymmetricChannelKeys: vi.fn(async () => JSON.stringify({
    version: 1,
    exportedAt: 1710000000000,
    channels: [{
      channelId: 'ch-1',
      keyVersion: 1,
      keyVersionId: 'kv-1',
      key: 'a2V5',
      acquiredAt: 1710000000000,
    }],
  }, null, 2)),
  exportSymmetricChannelKeys: vi.fn(),
  generateAndStoreDeviceKeypair: vi.fn(),
  getDeviceKeySummary: vi.fn(async () => ({
    hasKeypair: true,
    kemPublicKeyPreview: 'kem-preview',
    dsaPublicKeyPreview: 'dsa-preview',
    hasSigningKeypair: true,
  })),
  KeypairDeviceIdExistsError: class KeypairDeviceIdExistsError extends Error {},
  importAndStoreDeviceKeypair: vi.fn(),
  importAsymmetricChannelKeys: vi.fn(async () => 1),
  importSymmetricChannelKeys: vi.fn(),
  listDeviceKeypairs: vi.fn(async () => [{ deviceId: 'dev-1', createdAt: 1, updatedAt: 2 }]),
  listChannelKeys: vi.fn(async () => [{
    channelId: 'ch-1',
    keyVersion: 1,
    keyVersionId: 'kv-1',
    type: 'asymmetric',
    acquiredAt: 1710000000000,
    expiresAt: null,
  }]),
  listSymmetricChannelKeys: vi.fn(async () => []),
}))

vi.mock('../../lib/api', () => ({
  userDevicesList: vi.fn(async () => ({
    success: true,
    data: [{
      deviceId: 'dev-1',
      label: 'Portatil',
      publicKey: 'kem-raw',
      kemPublicKey: 'kem-raw',
      dsaPublicKey: 'dsa-raw',
      hasPublicKey: true,
      hasKemPublicKey: true,
      hasDsaPublicKey: true,
      createdAt: '2026-01-01T00:00:00Z',
      lastSeen: '2026-05-23T00:00:00Z',
      revoked: false,
      isCurrent: true,
    }],
  })),
  userDeviceRevoke: vi.fn(),
}))

describe('DeviceKeysModal', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('mostra l estat separat de claus KEM i DSA del dispositiu actiu', async () => {
    render(
      <DeviceKeysModal
        isOpen
        onClose={() => {}}
        currentDeviceId="dev-1"
        channels={[{ channelId: 'ch-1', name: 'general' }]}
        devices={[{ deviceId: 'dev-1', label: 'Portatil', revoked: false, lastSeen: '2026-05-23T00:00:00Z' }]}
      />
    )

    await waitFor(() => {
      expect(screen.getByText(/Signing key local:/)).toBeInTheDocument()
    })

    expect(screen.getByText('KEM public key: kem-preview')).toBeInTheDocument()
    expect(screen.getByText('DSA public key: dsa-preview')).toBeInTheDocument()
    expect(screen.getByText(/KEM: registrada/)).toBeInTheDocument()
    expect(screen.getByText(/DSA: registrada/)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('tab', { name: 'Gestió canals' }))
    expect(screen.getByText(/Bundles asimètrics de canals/)).toBeInTheDocument()
    expect(screen.getByText(/Canal general · ch-1 · v1/)).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Exportar asimètriques' }))
    await waitFor(() => {
      expect(screen.getByText(/Backup de claus asimètriques/)).toBeInTheDocument()
    })

    const asymImport = screen.getByPlaceholderText('Enganxa aquí JSON de bundles asimètrics')
    fireEvent.change(asymImport, { target: { value: '{"version":1,"exportedAt":1,"channels":[]}' } })
    fireEvent.click(screen.getByRole('button', { name: 'Importar asimètriques' }))
    await waitFor(() => {
      expect(screen.getByText(/Importades 1 claus asimètriques de canals/)).toBeInTheDocument()
    })
  })
})