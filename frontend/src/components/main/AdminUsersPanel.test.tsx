import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import '@testing-library/jest-dom'

import { AdminUsersPanel } from './AdminUsersPanel'

vi.mock('../../lib/api', () => ({
  adminPlansCreate: vi.fn(),
  adminPlansDelete: vi.fn(),
  adminPlansList: vi.fn(),
  adminPlansUpdate: vi.fn(),
  adminServersCreate: vi.fn(),
  adminServersDelete: vi.fn(),
  adminServersList: vi.fn(),
  adminServersUpdate: vi.fn(),
  adminUserLimitsGet: vi.fn(),
  adminUsersCreate: vi.fn(),
  adminUsersDelete: vi.fn(),
  adminUsersList: vi.fn(),
  adminUsersUpdatePlan: vi.fn(),
  adminUsersUpdateRole: vi.fn(),
  invitationsCreate: vi.fn(),
  invitationsList: vi.fn(),
}))

import {
  adminPlansCreate,
  adminPlansList,
  adminServersList,
  adminUsersList,
  invitationsList,
} from '../../lib/api'

const mockAdminUsersList = vi.mocked(adminUsersList)
const mockInvitationsList = vi.mocked(invitationsList)
const mockAdminServersList = vi.mocked(adminServersList)
const mockAdminPlansList = vi.mocked(adminPlansList)
const mockAdminPlansCreate = vi.mocked(adminPlansCreate)

const basePlans = [
  {
    id: '550e8400-e29b-41d4-a716-446655441001',
    name: 'free',
    displayName: 'Free',
    description: 'Plan gratuït',
    maxServers: 1,
    maxChannelsTextPerServer: 3,
    maxChannelsVoicePerServer: 2,
    maxMembersPerServer: 20,
    apiCallsPerMinute: 60,
    messagesPerDay: 10000,
    isSystem: true,
  },
]

describe('AdminUsersPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks()

    mockAdminUsersList.mockResolvedValue({ success: true, data: [] })
    mockInvitationsList.mockResolvedValue({ success: true, data: [] })
    mockAdminServersList.mockResolvedValue({ success: true, data: [] })
    mockAdminPlansList.mockResolvedValue({ success: true, data: basePlans })
    mockAdminPlansCreate.mockResolvedValue({
      success: true,
      data: {
        id: '550e8400-e29b-41d4-a716-446655449001',
        name: 'team_plus',
        displayName: 'Team Plus',
        description: 'Plan per equips',
        maxServers: 8,
        maxChannelsTextPerServer: 30,
        maxChannelsVoicePerServer: 15,
        maxMembersPerServer: 800,
        apiCallsPerMinute: 1200,
        messagesPerDay: -1,
        isSystem: false,
      },
    })
  })

  it('mostra la pestanya Plans amb plans carregats', async () => {
    render(
      <AdminUsersPanel
        isOpen={true}
        onClose={() => undefined}
        onFeedback={() => undefined}
      />
    )

    await waitFor(() => {
      expect(mockAdminPlansList).toHaveBeenCalled()
    })

    fireEvent.click(screen.getByRole('button', { name: 'Plans' }))

    await waitFor(() => {
      expect(screen.getByText('Plans (1)')).toBeInTheDocument()
    })

    expect(screen.getByText('Free (free)')).toBeInTheDocument()
  })

  it('permet crear un pla des de la pestanya Plans', async () => {
    const onFeedback = vi.fn()

    render(
      <AdminUsersPanel
        isOpen={true}
        onClose={() => undefined}
        onFeedback={onFeedback}
      />
    )

    await waitFor(() => {
      expect(mockAdminPlansList).toHaveBeenCalled()
    })

    fireEvent.click(screen.getByRole('button', { name: 'Plans' }))

    fireEvent.change(screen.getByLabelText('Nom intern'), { target: { value: 'team_plus' } })
    fireEvent.change(screen.getByLabelText('Nom visible'), { target: { value: 'Team Plus' } })
    fireEvent.change(screen.getByLabelText('Descripció (opcional)'), { target: { value: 'Plan per equips' } })
    fireEvent.change(screen.getByLabelText('Max servidors'), { target: { value: '8' } })
    fireEvent.change(screen.getByLabelText('Max canals text'), { target: { value: '30' } })
    fireEvent.change(screen.getByLabelText('Max canals veu'), { target: { value: '15' } })
    fireEvent.change(screen.getByLabelText('Max membres'), { target: { value: '800' } })
    fireEvent.change(screen.getByLabelText('API calls/min'), { target: { value: '1200' } })
    fireEvent.change(screen.getByLabelText('Msgs/dia'), { target: { value: '-1' } })

    fireEvent.click(screen.getByRole('button', { name: 'Crear pla' }))

    await waitFor(() => {
      expect(mockAdminPlansCreate).toHaveBeenCalledWith({
        name: 'team_plus',
        displayName: 'Team Plus',
        description: 'Plan per equips',
        maxServers: 8,
        maxChannelsTextPerServer: 30,
        maxChannelsVoicePerServer: 15,
        maxMembersPerServer: 800,
        apiCallsPerMinute: 1200,
        messagesPerDay: -1,
      })
    })

    expect(onFeedback).toHaveBeenCalledWith('Pla Team Plus creat')
  })
})
