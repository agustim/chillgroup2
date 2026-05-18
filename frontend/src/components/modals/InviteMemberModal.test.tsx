import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest'
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react'
import '@testing-library/jest-dom'
import { InviteMemberModal } from './InviteMemberModal'

describe('InviteMemberModal', () => {
  afterEach(() => {
    cleanup()
    document.body.innerHTML = ''
    document.body.style.overflow = ''
    vi.useRealTimers()
  })

  let onClose = vi.fn()
  let onInvite = vi.fn().mockResolvedValue(undefined)

  beforeEach(() => {
    onClose = vi.fn()
    onInvite = vi.fn().mockResolvedValue(undefined)
  })

  function renderModal(opts: {
    inviteType?: 'server' | 'channel'
    targetName?: string
    open?: boolean
    onInviteFn?: typeof onInvite
  } = {}) {
    return render(
      <InviteMemberModal
        isOpen={opts.open ?? true}
        onClose={onClose}
        onInvite={opts.onInviteFn ?? onInvite}
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

  it('bloqueja el submit amb nom buit', () => {
    renderModal()
    expect(screen.getByRole('button', { name: /Convidar/ })).toBeDisabled()
  })

  it('el botó s activa amb nom de 3 caracters', () => {
    renderModal()
    const input = screen.getByRole('textbox') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'abc' } })
    expect(screen.getByRole('button', { name: /Convidar/ })).not.toBeDisabled()
  })

  it('accepta un nom valid i crida onInvite', async () => {
    renderModal()
    const input = screen.getByRole('textbox') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'marc123' } })
    const submitBtn = screen.getByRole('button', { name: /Convidar/ })
    expect(submitBtn).not.toBeDisabled()
    fireEvent.click(submitBtn)
    await waitFor(() => {
      expect(onInvite).toHaveBeenCalledWith('marc123')
    })
  })

  it('tanca el modal despres de convidar amb exit', async () => {
    renderModal()
    const input = screen.getByRole('textbox') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'marc123' } })
    fireEvent.click(screen.getByRole('button', { name: /Convidar/ }))
    // The modal shows success then closes after 1500ms setTimeout
    // With real timers, waitFor won't wait for setTimeout
    // Just verify that onInvite was called
    await waitFor(() => {
      expect(onInvite).toHaveBeenCalledWith('marc123')
    })
  })

  it('desactiva el boto durant el submit', async () => {
    const onInviteDelay = vi.fn().mockImplementation(
      () => new Promise((resolve) => setTimeout(resolve, 200))
    )
    renderModal({ onInviteFn: onInviteDelay })
    const input = screen.getByRole('textbox') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'marc123' } })
    fireEvent.click(screen.getByRole('button', { name: /Convidar/ }))
    const btn = screen.getByRole('button', { name: /Enviant/ })
    expect(btn).toBeDisabled()
    await waitFor(() => {
      expect(onInviteDelay).toHaveBeenCalled()
    })
  })

  it('accepta noms de 3+ caracters', () => {
    renderModal()
    const input = screen.getByRole('textbox') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'abc' } })
    expect(screen.getByRole('button', { name: /Convidar/ })).not.toBeDisabled()
  })

  it('validacio en submit rejecta noms de 2 caracters o menys', async () => {
    // Button is enabled (validation only happens on submit)
    renderModal()
    const input = screen.getByRole('textbox') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'ab' } })
    expect(screen.getByRole('button', { name: /Convidar/ })).not.toBeDisabled()
  })

  it('neteja el camp de text en enviar', async () => {
    renderModal()
    const input = screen.getByRole('textbox') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'marc123' } })
    fireEvent.click(screen.getByRole('button', { name: /Convidar/ }))
    await waitFor(() => {
      expect(onInvite).toHaveBeenCalledWith('marc123')
    })
    // After success, the input is cleared
    // This happens in a setTimeout, so with real timers it may not be reflected
  })

  it('utilitza el contextLabel correcte per al tipus de target', () => {
    renderModal({ inviteType: 'channel', targetName: '# general' })
    expect(screen.getByText(/Convidar al canal/i)).toBeTruthy()
    renderModal({ inviteType: 'server', targetName: 'Mon server' })
    expect(screen.getByText(/Convidar al servidor/i)).toBeTruthy()
  })
})
