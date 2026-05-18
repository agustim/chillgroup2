import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest'
import { render, screen, fireEvent, cleanup, waitFor, act } from '@testing-library/react'
import '@testing-library/jest-dom'
import { CreateServerModal } from './CreateServerModal'

describe('CreateServerModal', () => {
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
      <CreateServerModal
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
    expect(screen.getByText('Crear servidor')).toBeTruthy()
    expect(screen.getByLabelText('Nom del servidor')).toBeTruthy()
    expect(screen.getByLabelText('URL de la icona (opcional)')).toBeTruthy()
  })

  it('el botó de crear esta disabled quan el nom esta buit', () => {
    renderModal()
    expect(screen.getByRole('button', { name: /Crear/ })).toBeDisabled()
  })

  it('el botó s activa amb nom de 2+ caracters', () => {
    renderModal()
    const input = screen.getByLabelText('Nom del servidor') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'AB' } })
    expect(screen.getByRole('button', { name: /Crear/ })).not.toBeDisabled()
  })

  it('el botó s activa amb nom de 1 caracter (validacio en submit)', () => {
    renderModal()
    const input = screen.getByLabelText('Nom del servidor') as HTMLInputElement
    // Validation only happens on submit, not on input change
    fireEvent.change(input, { target: { value: 'A' } })
    expect(screen.getByRole('button', { name: /Crear/ })).not.toBeDisabled()
  })

  it('accepta un nom valid i crida onCreate', async () => {
    renderModal()
    const input = screen.getByLabelText('Nom del servidor') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'El meu servidor' } })
    fireEvent.click(screen.getByRole('button', { name: /Crear/ }))
    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('El meu servidor', null)
    })
  })

  it('neteja el formulari i tanca despres de crear amb exit', async () => {
    renderModal()
    const input = screen.getByLabelText('Nom del servidor') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'Nou Servidor' } })
    fireEvent.click(screen.getByRole('button', { name: /Crear/ }))
    // Mock resolves immediately, so onClose is called
    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('Nou Servidor', null)
    })
    await waitFor(() => {
      expect(onClose).toHaveBeenCalledTimes(1)
    })
  })

  it('tanca el modal amb el botó Cancel·lar', () => {
    renderModal()
    fireEvent.click(screen.getByRole('button', { name: /Cancel·lar/ }))
    expect(onClose).toHaveBeenCalledTimes(1)
    expect(onCreate).not.toHaveBeenCalled()
  })

  it('bloqueja submit amb nom buit', () => {
    renderModal()
    const btn = screen.getByRole('button', { name: /Crear/ })
    expect(btn).toBeDisabled()
    fireEvent.click(btn)
    expect(onCreate).not.toHaveBeenCalled()
  })

  it('validacio en submit rejecta nom curt', async () => {
    renderModal()
    const input = screen.getByLabelText('Nom del servidor') as HTMLInputElement
    // Button is enabled (validation only happens on submit)
    fireEvent.change(input, { target: { value: 'A' } })
    expect(screen.getByRole('button', { name: /Crear/ })).not.toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: /Crear/ }))
    // The validation error is shown on submit
    await waitFor(() => {
      expect(onCreate).not.toHaveBeenCalled()
    })
  })

  it('accepta nom amb icona', async () => {
    renderModal()
    const nameInput = screen.getByLabelText('Nom del servidor') as HTMLInputElement
    fireEvent.change(nameInput, { target: { value: 'Servidor' } })
    const iconInput = screen.getByLabelText('URL de la icona (opcional)') as HTMLInputElement
    fireEvent.change(iconInput, { target: { value: 'https://example.com/icon.png' } })
    fireEvent.click(screen.getByRole('button', { name: /Crear/ }))
    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('Servidor', 'https://example.com/icon.png')
    })
  })

  it('passa null per a icona quan esta buit', async () => {
    renderModal()
    const input = screen.getByLabelText('Nom del servidor') as HTMLInputElement
    fireEvent.change(input, { target: { value: 'Servidor' } })
    fireEvent.click(screen.getByRole('button', { name: /Crear/ }))
    await waitFor(() => {
      expect(onCreate).toHaveBeenCalledWith('Servidor', null)
    })
  })
})
