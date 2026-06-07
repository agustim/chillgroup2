import React, { useRef, useState } from 'react'
import { useAuth } from '../contexts/AuthContext'
import { Button } from './shared/Button'
import { importFullBackup, decryptBackup, isEncryptedBackup } from '../lib/device-keys'

export function LoginScreen() {
  const { login, register, registerWithInvitation, isLoading, error } = useAuth()
  const openRegisterEnv = (__OPEN_REGISTER__ ?? 'true').toString().toLowerCase()
  const initialOpenRegister = openRegisterEnv === 'true' || openRegisterEnv === '1' || openRegisterEnv === 'yes' || openRegisterEnv === 'on'
  const urlInvite = new URLSearchParams(window.location.search).get('invite') ?? ''
  const [isLogin, setIsLogin] = useState(urlInvite ? false : true)
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [isOpenRegister, setIsOpenRegister] = useState(initialOpenRegister)
  const [invitationCode, setInvitationCode] = useState(urlInvite)
  const [validationError, setValidationError] = useState('')
  const [backupStatus, setBackupStatus] = useState<{ type: 'success' | 'error'; message: string } | null>(null)
  const [isImportingBackup, setIsImportingBackup] = useState(false)
  const [pendingEncryptedBackup, setPendingEncryptedBackup] = useState<string | null>(null)
  const [backupPassword, setBackupPassword] = useState('')
  const backupFileRef = useRef<HTMLInputElement>(null)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setValidationError('')

    if (!username.trim()) {
      setValidationError('Introdueix el nom d\'usuari')
      return
    }

    if (!password) {
      setValidationError('Introdueix la contrasenya')
      return
    }

    if (!isLogin && password.length < 8) {
      setValidationError('La contrasenya ha de tenir almenys 8 caràcters')
      return
    }

    if (!isLogin && !confirmPassword) {
      setValidationError('Confirma la contrasenya')
      return
    }

    if (!isLogin && password !== confirmPassword) {
      setValidationError('Les contrasenyes no coincideixen')
      return
    }

    const invitation = invitationCode.trim()

    if (!isLogin && !isOpenRegister && !invitation) {
      setValidationError('Introdueix el codi d\'invitació')
      return
    }

    try {
      if (isLogin) {
        await login(username, password)
      } else {
        if (!isOpenRegister || invitation) {
          await registerWithInvitation(invitation, username, password)
        } else {
          await register(username, password)
        }
      }
    } catch (err) {
      if (err instanceof Error && err.message.toLowerCase().includes('registre està tancat')) {
        setIsOpenRegister(false)
      }
    }
  }

  const handleToggle = () => {
    setIsLogin(!isLogin)
    setValidationError('')
    setUsername('')
    setPassword('')
    setConfirmPassword('')
    setInvitationCode('')
  }

  const handleBackupFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    setBackupStatus(null)
    setBackupPassword('')
    try {
      const text = await file.text()
      if (isEncryptedBackup(text)) {
        setPendingEncryptedBackup(text)
        setBackupStatus(null)
      } else {
        setIsImportingBackup(true)
        const result = await importFullBackup(text)
        setBackupStatus({
          type: 'success',
          message: `Backup restaurat: ${result.devices} dispositiu(s), ${result.symmetricChannels + result.asymmetricChannels} clau(s) de canal`,
        })
      }
    } catch (err) {
      setBackupStatus({
        type: 'error',
        message: err instanceof Error ? err.message : 'No s\'ha pogut llegir el fitxer',
      })
    } finally {
      setIsImportingBackup(false)
      if (backupFileRef.current) backupFileRef.current.value = ''
    }
  }

  const handleDecryptAndImport = async () => {
    if (!pendingEncryptedBackup || !backupPassword) return
    setIsImportingBackup(true)
    setBackupStatus(null)
    try {
      const plaintext = await decryptBackup(pendingEncryptedBackup, backupPassword)
      const result = await importFullBackup(plaintext)
      setPendingEncryptedBackup(null)
      setBackupPassword('')
      setBackupStatus({
        type: 'success',
        message: `Backup restaurat: ${result.devices} dispositiu(s), ${result.symmetricChannels + result.asymmetricChannels} clau(s) de canal`,
      })
    } catch (err) {
      setBackupStatus({
        type: 'error',
        message: err instanceof Error ? err.message : 'No s\'ha pogut importar el backup',
      })
    } finally {
      setIsImportingBackup(false)
    }
  }

  const displayError = error || validationError

  return (
    <div className="login-screen">
      <div className="login-container">
        <div className="login-header">
          <h1>ChillGroup v2</h1>
          <p>Missatgeria segura amb encriptació E2EE</p>
        </div>

        <form onSubmit={handleSubmit} className="login-form">
          <div className="form-group">
            <label htmlFor="username">Nom d'usuari</label>
            <input
              id="username"
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="Nom d'usuari"
              autoComplete="username"
              required
              disabled={isLoading}
              autoFocus
            />
          </div>

          <div className="form-group">
            <label htmlFor="password">Contrasenya</label>
            <input
              id="password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="••••••••"
              autoComplete={isLogin ? 'current-password' : 'new-password'}
              required
              disabled={isLoading}
            />
            {!isLogin && password.length > 0 && password.length < 8 && (
              <span className="password-hint">Mínim 8 caràcters</span>
            )}
          </div>

          {!isLogin && (
            <div className="form-group">
              <label htmlFor="confirm-password">Confirmar contrasenya</label>
              <input
                id="confirm-password"
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                placeholder="••••••••"
                autoComplete="new-password"
                required
                disabled={isLoading}
              />
            </div>
          )}

          {!isLogin && (
            <div className="form-group">
              <label htmlFor="invitation-code">
                Codi d'invitació {isOpenRegister ? '(opcional)' : ''}
              </label>
              <input
                id="invitation-code"
                type="text"
                value={invitationCode}
                onChange={(e) => setInvitationCode(e.target.value)}
                placeholder="ABC123-DEF456-GHI789"
                autoComplete="off"
                required={!isOpenRegister}
                disabled={isLoading}
              />
              {!isOpenRegister && (
                <span className="password-hint">
                  El registre està tancat. Necessites una invitació per crear compte.
                </span>
              )}
            </div>
          )}

          {displayError && <div className="error-message">{displayError}</div>}

          <div className="form-actions">
            <Button type="submit" size="lg" disabled={isLoading}>
              {isLoading ? 'Carregant...' : isLogin ? 'Entrar' : 'Registrar-se'}
            </Button>
          </div>
        </form>

        <div className="login-footer">
          <button
            type="button"
            className="toggle-auth"
            onClick={handleToggle}
            disabled={isLoading}
          >
            {isLogin
              ? 'No tens compte? Registrar-se'
              : 'Ja tens compte? Entrar'}
          </button>
        </div>

        <div className="login-backup-section">
          <p className="login-backup-label">Tens un backup de claus?</p>
          {backupStatus && (
            <div className={backupStatus.type === 'success' ? 'modal-success' : 'modal-error'} style={{ marginBottom: '8px' }}>
              {backupStatus.message}
            </div>
          )}
          <input
            ref={backupFileRef}
            type="file"
            accept=".json,application/json"
            style={{ display: 'none' }}
            onChange={(e) => void handleBackupFileChange(e)}
            disabled={isImportingBackup || isLoading}
          />
          {pendingEncryptedBackup ? (
            <div className="backup-decrypt-form">
              <p className="login-backup-label" style={{ marginBottom: '6px' }}>
                El backup està xifrat. Introdueix la contrasenya:
              </p>
              <input
                type="password"
                className="form-input"
                value={backupPassword}
                onChange={(e) => setBackupPassword(e.target.value)}
                onKeyDown={(e) => { if (e.key === 'Enter') void handleDecryptAndImport() }}
                placeholder="Contrasenya del backup"
                autoComplete="current-password"
                disabled={isImportingBackup}
                autoFocus
              />
              <div style={{ display: 'flex', gap: '8px', marginTop: '8px', justifyContent: 'center' }}>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => { setPendingEncryptedBackup(null); setBackupPassword('') }}
                  disabled={isImportingBackup}
                >
                  Cancel·lar
                </Button>
                <Button
                  variant="primary"
                  size="sm"
                  onClick={() => void handleDecryptAndImport()}
                  disabled={isImportingBackup || !backupPassword}
                >
                  {isImportingBackup ? 'Desxifrant...' : 'Importar'}
                </Button>
              </div>
            </div>
          ) : (
            <Button
              variant="secondary"
              size="sm"
              onClick={() => backupFileRef.current?.click()}
              disabled={isImportingBackup || isLoading}
            >
              {isImportingBackup ? 'Important...' : 'Restaurar backup (.json)'}
            </Button>
          )}
        </div>
      </div>
    </div>
  )
}