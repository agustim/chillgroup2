import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { Modal } from '../ui/Modal'

describe('Modal genèric', () => {
  it('no renderitza res quan isOpen es false', () => {
    const { unmount } = render(
      <Modal isOpen={false} onClose={vi.fn()} title="Test">
        <p>Contingut</p>
      </Modal>
    )
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(document.body.style.overflow).toBe('')
    unmount()
  })

  it('renderitza el modal quan isOpen es true', () => {
    const onClose = vi.fn()
    const { unmount } = render(
      <Modal isOpen={true} onClose={onClose} title="Títol de prova">
        <p>Contingut del modal</p>
      </Modal>
    )
    expect(screen.getByRole('dialog')).toBeTruthy()
    expect(screen.getByText('Títol de prova')).toBeTruthy()
    expect(screen.getByText('Contingut del modal')).toBeTruthy()
    expect(document.body.style.overflow).toBe('hidden')
    unmount()
  })

  it('tanca el modal en prémer el botó de tancar (X)', () => {
    const onClose = vi.fn()
    const { unmount } = render(
      <Modal isOpen={true} onClose={onClose} title="Test">
        <p>Contingut</p>
      </Modal>
    )
    const closeButton = screen.getByLabelText('Tancar')
    fireEvent.click(closeButton)
    expect(onClose).toHaveBeenCalledTimes(1)
    unmount()
  })

  it('tanca el modal en prémer ESC', () => {
    const onClose = vi.fn()
    const { unmount } = render(
      <Modal isOpen={true} onClose={onClose} title="Test">
        <p>Contingut</p>
      </Modal>
    )
    const handleSpy = vi.fn()
    document.addEventListener('keydown', handleSpy)
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(onClose).toHaveBeenCalledTimes(1)
    document.removeEventListener('keydown', handleSpy)
    unmount()
  })

  it('NO tanca el modal en prémer una tecla diferent de ESC', () => {
    const onClose = vi.fn()
    const { unmount } = render(
      <Modal isOpen={true} onClose={onClose} title="Test">
        <p>Contingut</p>
      </Modal>
    )
    const handleSpy = vi.fn()
    document.addEventListener('keydown', handleSpy)
    fireEvent.keyDown(document, { key: 'Enter' })
    expect(onClose).not.toHaveBeenCalled()
    document.removeEventListener('keydown', handleSpy)
    unmount()
  })

  it('tanca el modal en clicar l overlay', () => {
    const onClose = vi.fn()
    const { container, unmount } = render(
      <Modal isOpen={true} onClose={onClose} title="Test">
        <p>Contingut</p>
      </Modal>
    )
    const overlay = container.querySelector('.modal-overlay') as HTMLElement
    fireEvent.click(overlay)
    expect(onClose).toHaveBeenCalledTimes(1)
    unmount()
  })

  it('NO tanca el modal en clicar dins del dialòg', () => {
    const onClose = vi.fn()
    const { unmount } = render(
      <Modal isOpen={true} onClose={onClose} title="Test">
        <button>Botó intern</button>
      </Modal>
    )
    const button = screen.getByText('Botó intern')
    fireEvent.click(button)
    expect(onClose).not.toHaveBeenCalled()
    unmount()
  })

  it('estableix aria-modal i aria-label correctes', () => {
    const { container, unmount } = render(
      <Modal isOpen={true} onClose={vi.fn()} title="Test Modal">
        <p>Contingut</p>
      </Modal>
    )
    const dialog = container.querySelector('.modal-dialog')
    expect(dialog?.getAttribute('aria-modal')).toBe('true')
    expect(dialog?.getAttribute('aria-label')).toBe('Test Modal')
    unmount()
  })
})
