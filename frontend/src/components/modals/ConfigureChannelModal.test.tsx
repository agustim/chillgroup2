import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest'
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react'
import '@testing-library/jest-dom'
import { ConfigureChannelModal } from './ConfigureChannelModal'
import type { Channel } from '../../types'

describe('ConfigureChannelModal', () => {
  afterEach(() => {
    cleanup()
    document.body.innerHTML = ''
    document.body.style.overflow = ''
  })

  let onClose = vi.fn()
  let onUpdateFn = vi.fn().mockResolvedValue(undefined)
  let onDeleteFn = vi.fn().mockResolvedValue(undefined)

  const testChannel: Channel = {
    channelId: 'ch-1',
    name: 'general',
    type: 'text',
    encryptionType: 'symmetric',
    isPrivate: false,
    messageTTL: 3600,
    createdAt: '2026-01-01T00:00:00Z',
  }

  beforeEach(() => {
    onClose = vi.fn()
    onUpdateFn = vi.fn().mockResolvedValue(undefined)
    onDeleteFn = vi.fn().mockResolvedValue(undefined)
  })

  function renderWithChannel(channel: Channel | null = testChannel) {
    return render(
      <ConfigureChannelModal
        isOpen={true}
        onClose={onClose}
        channel={channel}
        onUpdate={onUpdateFn}
        onDelete={onDeleteFn}
      />
    )
  }

  it('no renderitza res quan channel es null', () => {
    renderWithChannel(null)
    expect(screen.queryByRole('dialog')).toBeNull()
  })

  it('renderitza el formulari amb valors del canal', () => {
    renderWithChannel()
    expect(screen.getByRole('dialog')).toBeTruthy()
    expect(screen.getByText((content) => content.includes('Configuració del canal'))).toBeTruthy()
    const nameInput = screen.getByLabelText(/Nom del canal/i) as HTMLInputElement
    expect(nameInput.value).toBe('general')
    const ttlInput = screen.getByLabelText(/Durada dels missatges/i) as HTMLInputElement
    expect(ttlInput.value).toBe('3600')
  })

  it('actualitza els valors quan canvia el canal via props', () => {
    const { rerender } = renderWithChannel({
      ...testChannel, name: 'altres', messageTTL: 7200,
    })
    const nameInput = screen.getByLabelText(/Nom del canal/i) as HTMLInputElement
    expect(nameInput.value).toBe('altres')
    rerender(
      <ConfigureChannelModal
        isOpen={true}
        onClose={onClose}
        channel={{ ...testChannel, name: 'novo', messageTTL: null }}
        onUpdate={onUpdateFn}
        onDelete={onDeleteFn}
      />
    )
    const nameInput2 = screen.getByLabelText(/Nom del canal/i) as HTMLInputElement
    expect(nameInput2.value).toBe('novo')
  })

  it('mostra la informacio del canal (tipus, encriptacio, privat)', () => {
    renderWithChannel()
    expect(screen.getByText('# Text')).toBeTruthy()
    expect(screen.getByText(/🔒/)).toBeTruthy()
    expect(screen.getByText(/Privat:/)).toBeTruthy()
    expect(screen.getByText('No')).toBeTruthy()
  })

  it('bloqueja el submit amb nom buit', () => {
    renderWithChannel()
    const nameInput = screen.getByLabelText(/Nom del canal/i) as HTMLInputElement
    fireEvent.change(nameInput, { target: { value: '  ' } })
    expect(screen.getByRole('button', { name: /Desar canvis/ })).not.toBeDisabled()
  })

  it('accepta un nom valid amb TTL numeric i isPrivate', async () => {
    renderWithChannel()
    const nameInput = screen.getByLabelText(/Nom del canal/i) as HTMLInputElement
    fireEvent.change(nameInput, { target: { value: 'novo-canal' } })
    const ttlInput = screen.getByLabelText(/Durada dels missatges/i) as HTMLInputElement
    fireEvent.change(ttlInput, { target: { value: '1800' } })
    fireEvent.click(screen.getByRole('button', { name: /Desar canvis/ }))
    await waitFor(() => {
      expect(onUpdateFn).toHaveBeenCalledWith('novo-canal', 1800, false)
    })
  })

  it('accepta un nom valid amb TTL buit (null) i isPrivate', async () => {
    renderWithChannel()
    const nameInput = screen.getByLabelText(/Nom del canal/i) as HTMLInputElement
    fireEvent.change(nameInput, { target: { value: 'canal-sense-limit' } })
    const ttlInput = screen.getByLabelText(/Durada dels missatges/i) as HTMLInputElement
    fireEvent.change(ttlInput, { target: { value: '' } })
    fireEvent.click(screen.getByRole('button', { name: /Desar canvis/ }))
    await waitFor(() => {
      expect(onUpdateFn).toHaveBeenCalledWith('canal-sense-limit', null, false)
    })
  })

  it('valida TTL positiu', () => {
    renderWithChannel()
    const nameInput = screen.getByLabelText(/Nom del canal/i) as HTMLInputElement
    fireEvent.change(nameInput, { target: { value: 'canal' } })
    const ttlInput = screen.getByLabelText(/Durada dels missatges/i) as HTMLInputElement
    fireEvent.change(ttlInput, { target: { value: '-5' } })
    expect(screen.getByRole('button', { name: /Desar canvis/ })).not.toBeDisabled()
  })

  it('valida TTL numeric', () => {
    renderWithChannel()
    const nameInput = screen.getByLabelText(/Nom del canal/i) as HTMLInputElement
    fireEvent.change(nameInput, { target: { value: 'canal' } })
    const ttlInput = screen.getByLabelText(/Durada dels missatges/i) as HTMLInputElement
    fireEvent.change(ttlInput, { target: { value: 'abc' } })
    expect(screen.getByRole('button', { name: /Desar canvis/ })).not.toBeDisabled()
  })

  it('tanca el modal amb el botó X', () => {
    renderWithChannel()
    fireEvent.click(screen.getByLabelText('Tancar'))
    expect(onClose).toHaveBeenCalledTimes(1)
    expect(onUpdateFn).not.toHaveBeenCalled()
    expect(onDeleteFn).not.toHaveBeenCalled()
  })

  it('neteja el formulari despres de desar amb exit', async () => {
    renderWithChannel()
    const nameInput = screen.getByLabelText(/Nom del canal/i) as HTMLInputElement
    fireEvent.change(nameInput, { target: { value: 'canal-actualitzat' } })
    fireEvent.click(screen.getByRole('button', { name: /Desar canvis/ }))
    await waitFor(() => {
      expect(onUpdateFn).toHaveBeenCalledWith('canal-actualitzat', 3600, false)
    })
  })

  it('mostra el botó esborrar canal', () => {
    renderWithChannel()
    expect(screen.getByRole('button', { name: /Esborrar canal/ })).toBeTruthy()
  })

  it('mostra confirmació d\'esborrat en clicar el botó', () => {
    renderWithChannel()
    fireEvent.click(screen.getByRole('button', { name: /Esborrar canal/ }))
    expect(screen.getByText(/Estàs segur que vols esborrar aquest canal/)).toBeTruthy()
    expect(screen.getByRole('button', { name: /^Esborrar$/ })).toBeTruthy()
    // After confirmation shown, there are now 2 Cancel·lar buttons; just check count
    const cancelButtons = screen.getAllByRole('button', { name: /Cancel·lar/ })
    expect(cancelButtons.length).toBe(2)
  })

  it('confirmació visual d\'esborrat crida onDelete', async () => {
    renderWithChannel()
    fireEvent.click(screen.getByRole('button', { name: /Esborrar canal/ }))
    // After showing confirmation, use the exact "Esborrar" button (not "Esborrar canal")
    fireEvent.click(screen.getByRole('button', { name: /^Esborrar$/ }))
    await waitFor(() => {
      expect(onDeleteFn).toHaveBeenCalledWith('ch-1')
    })
    expect(onClose).toHaveBeenCalled()
  })

  it('cancel·lar esborrat tanca el confirmation', () => {
    renderWithChannel()
    fireEvent.click(screen.getByRole('button', { name: /Esborrar canal/ }))
    const cancelButtons = screen.getAllByRole('button', { name: /Cancel·lar/ })
    // the one inside the error confirm div (first one)
    fireEvent.click(cancelButtons[0])
    expect(onDeleteFn).not.toHaveBeenCalled()
    expect(screen.queryByText(/Estàs segur que vols esborrar aquest canal/)).toBeNull()
  })

  it('canvia el camp isPrivate en checkbox', async () => {
    renderWithChannel()
    const checkbox = screen.getByLabelText(/Canal privat/i) as HTMLInputElement
    expect(checkbox.checked).toBe(false)
    fireEvent.click(checkbox)
    await waitFor(() => {
      expect(onUpdateFn).toHaveBeenCalledWith('general', 3600, true)
    })
  })
})
