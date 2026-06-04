import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest'
import { render, screen, fireEvent, cleanup, waitFor, act } from '@testing-library/react'
import '@testing-library/jest-dom'
import { InviteMemberModal } from './InviteMemberModal'
import type { UserSearchResult } from '../../types'

const mockUser: UserSearchResult = {
  userId: 'u1',
  username: 'marc123',
  status: 'online',
  isFriend: false,
}

describe('InviteMemberModal', () => {
  afterEach(() => {
    cleanup()
    document.body.innerHTML = ''
    document.body.style.overflow = ''
    vi.useRealTimers()
  })

  let onClose = vi.fn()
  let onInvite = vi.fn().mockResolvedValue(undefined)
  let onSearchUsers = vi.fn().mockResolvedValue([mockUser])

  beforeEach(() => {
    onClose = vi.fn()
    onInvite = vi.fn().mockResolvedValue(undefined)
    onSearchUsers = vi.fn().mockResolvedValue([mockUser])
    vi.useFakeTimers()
  })

  function renderModal(opts: {
    inviteType?: 'server' | 'channel'
    targetName?: string
    open?: boolean
  } = {}) {
    return render(
      <InviteMemberModal
        isOpen={opts.open ?? true}
        onClose={onClose}
        onInvite={onInvite}
        onSearchUsers={onSearchUsers}
        inviteType={opts.inviteType ?? 'server'}
        targetName={opts.targetName ?? 'El meu servidor'}
      />
    )
  }

  it('no renderitza res quan isOpen es false', () => {
    renderModal({ open: false })
    expect(screen.queryByRole('dialog')).toBeNull()
  })

  it('mostra el titol correcte per a servidor', () => {
    renderModal({ inviteType: 'server', targetName: 'Servidor Test' })
    expect(screen.getByRole('dialog')).toBeTruthy()
    expect(screen.getByText(/Convidar al servidor/i)).toBeTruthy()
  })

  it('mostra el titol correcte per a canal', () => {
    renderModal({ inviteType: 'channel', targetName: '# general' })
    expect(screen.getByRole('dialog')).toBeTruthy()
    expect(screen.getByText(/Convidar al canal/i)).toBeTruthy()
  })

  it('mostra el nom del target', () => {
    renderModal({ targetName: 'Servidor Premium' })
    expect(screen.getByText(/Servidor Premium/)).toBeTruthy()
  })

  it('no cerca amb menys de 2 caracters', async () => {
    renderModal()
    const input = screen.getByRole('textbox') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'a' } })
    await act(async () => { await vi.runAllTimersAsync() })
    expect(onSearchUsers).not.toHaveBeenCalled()
  })

  it('cerca amb 2+ caracters despres del debounce', async () => {
    renderModal()
    const input = screen.getByRole('textbox') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'ma' } })
    await act(async () => { await vi.runAllTimersAsync() })
    expect(onSearchUsers).toHaveBeenCalledWith('ma')
  })

  it('mostra resultats de cerca', async () => {
    renderModal()
    const input = screen.getByRole('textbox') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'marc' } })
    await act(async () => { await vi.runAllTimersAsync() })
    expect(screen.getByText('marc123')).toBeTruthy()
  })

  it('crida onInvite amb el username en clicar Convidar', async () => {
    renderModal()
    const input = screen.getByRole('textbox') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'marc' } })
    await act(async () => { await vi.runAllTimersAsync() })
    screen.getByText('marc123')
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: /^Convidar$/ }))
      await vi.runAllTimersAsync()
    })
    expect(onInvite).toHaveBeenCalledWith('marc123')
  })

  it('utilitza el contextLabel correcte per al tipus de target', () => {
    renderModal({ inviteType: 'channel', targetName: '# general' })
    expect(screen.getByText(/Convidar al canal/i)).toBeTruthy()
  })
})
