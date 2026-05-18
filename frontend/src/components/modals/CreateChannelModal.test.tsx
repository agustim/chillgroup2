import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest'
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react'
import '@testing-library/jest-dom'
import { CreateChannelModal } from './CreateChannelModal'

describe('CreateChannelModal', () => {
  afterEach(() => {
    cleanup()
    document.body.innerHTML = ''
    document.body.style.overflow = ''
  })

  let onClose = vi.fn()
  let onCreate = vi.fn().mockResolvedValue(undefined)

  beforeEach(() => {
    onClose = vi.fn()
    onCreate = vi.fn().mockResolvedValue(undefined)
  })

  function renderModal(open = true) {
    return render(
      <CreateChannelModal
        isOpen={open}
        onClose={onClose}
        onCreate={onCreate}
      />
    )
  }

  it('no renderitza res quan isOpen es false', () => {
    renderModal(false)
    expect(screen.queryByRole('dialog')).toBeNull()
  })

  it('renderitza el formulari amb els camps correctes', () => {
    renderModal()
    expect(screen.getByRole('dialog')).toBeTruthy()
    expect(screen.getByText('Crear canal')).toBeTruthy()
    expect(screen.getByLabelText('Nom del canal')).toBeTruthy()
    expect(screen.getByText('# Text')).toBeTruthy()
    expect(screen.getByText('🔊 Veu')).toBeTruthy()
  })

  it('el botó de crear esta disabled quan el nom esta buit', () => {
    renderModal()
    expect(screen.getByRole('button', { name: /Crear/ })).toBeDisabled()
  })

  it('accepta un nom valid i crida onCreate com a text', async () => {
    renderModal()
    const input = screen.getByLabelText('Nom del canal') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'general' } })
    fireEvent.click(screen.getByRole('button', { name: /Crear/ }))
    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('general', 'text')
    })
    await waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1)
    })
  })

  it('converteix el nom a minuscules automaticament', () => {
    renderModal()
    const input = screen.getByLabelText('Nom del canal') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'GENERAL' } })
    expect(input.value).toBe('general')
  })

  it('rejecta noms amb espais en enviar', async () => {
    // The validation rejects names with spaces, onCreate should not be called
    renderModal()
    const input = screen.getByLabelText('Nom del canal') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'nom amb espais' } })
    fireEvent.click(screen.getByRole('button', { name: /Crear/ }))
    await waitFor(() => {
      expect(onCreate).not.toHaveBeenCalled()
    })
  })

  it('canvia el tipus de canal de text a veu', () => {
    renderModal()
    fireEvent.change(screen.getByLabelText('Nom del canal'), {
      target: { value: 'sortides' },
    })
    fireEvent.click(screen.getByText('🔊 Veu'))
    const voiceBtn = screen.getByText('🔊 Veu')
    expect(voiceBtn).toHaveClass('chillgroup-button--primary')
    const textBtn = screen.getByText('# Text')
    expect(textBtn).not.toHaveClass('chillgroup-button--primary')
  })

  it('el tipus text es l estandard', () => {
    renderModal()
    const textBtn = screen.getByText('# Text')
    expect(textBtn).toHaveClass('chillgroup-button--primary')
  })

  it('tanca el modal amb el botó Cancel·lar', () => {
    renderModal()
    fireEvent.click(screen.getByRole('button', { name: /Cancel·lar/ }))
    expect(onClose).toHaveBeenCalledTimes(1)
    expect(onCreate).not.toHaveBeenCalled()
  })

  it('neteja i tanca despres de crear', async () => {
    renderModal()
    const input = screen.getByLabelText('Nom del canal') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'nou-canal' } })
    fireEvent.click(screen.getByRole('button', { name: /Crear/ }))
    await waitFor(() => {
      expect(onCreate).toHaveBeenCalled()
    })
    await waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1)
    })
  })

  it('passa tipus veu quan es selecciona', async () => {
    renderModal()
    const input = screen.getByLabelText('Nom del canal') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'sortides' } })
    fireEvent.click(screen.getByText('🔊 Veu'))
    fireEvent.click(screen.getByRole('button', { name: /Crear/ }))
    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('sortides', 'voice')
    })
  })
})
