import React, { useState } from 'react'
import { useAuth } from '../contexts/AuthContext'
import { Button } from './shared/Button'

export function LoginScreen() {
  const { login, register, isLoading, error } = useAuth()
  const [isLogin, setIsLogin] = useState(true)
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [validationError, setValidationError] = useState('')

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

    try {
      if (isLogin) {
        await login(username, password)
      } else {
        await register(username, password)
      }
    } catch (err) {
      // L'error ja està gestionat per l'AuthContext
    }
  }

  const handleToggle = () => {
    setIsLogin(!isLogin)
    setValidationError('')
    setUsername('')
    setPassword('')
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
      </div>
    </div>
  )
}