import React from 'react'
import { AuthProvider, useAuth } from './contexts/AuthContext'
import { LoginScreen } from './components/LoginScreen'
import { AppLayout } from './components/AppLayout'
import { MobileLayout } from './components/MobileLayout'
import { DeviceUnlockScreen } from './components/DeviceUnlockScreen'
import { hasLocalVault, isLocalVaultUnlocked, lockLocalVault } from './lib/local-vault'
import { useIsMobile } from './hooks/useIsMobile'

function AppContent() {
  const { isAuthenticated, user, isLoading, logout } = useAuth()
  const isMobile = useIsMobile()
  const [vaultConfigured, setVaultConfigured] = React.useState<boolean>(() => hasLocalVault())
  const [vaultUnlocked, setVaultUnlocked] = React.useState<boolean>(() => isLocalVaultUnlocked())

  React.useEffect(() => {
    if (!isAuthenticated) {
      lockLocalVault()
      setVaultConfigured(hasLocalVault())
      setVaultUnlocked(false)
      return
    }

    setVaultConfigured(hasLocalVault())
    setVaultUnlocked(isLocalVaultUnlocked())
  }, [isAuthenticated])

  const handleVaultUnlocked = React.useCallback(() => {
    setVaultConfigured(hasLocalVault())
    setVaultUnlocked(isLocalVaultUnlocked())
  }, [])

  if (isLoading) {
    return (
      <div className="login-screen">
        <div className="login-container">
          <div className="login-header">
            <h1>ChillGroup v2</h1>
            <p>Carregant...</p>
          </div>
        </div>
      </div>
    )
  }

  if (isAuthenticated && user) {
    if (!vaultConfigured || !vaultUnlocked) {
      return (
        <DeviceUnlockScreen
          mode={vaultConfigured ? 'unlock' : 'setup'}
          username={user.username}
          onUnlocked={handleVaultUnlocked}
          onLogout={logout}
        />
      )
    }

    return isMobile
      ? <MobileLayout username={user.username} />
      : <AppLayout username={user.username} />
  }

  return <LoginScreen />
}

export default function App() {
  return (
    <AuthProvider>
      <AppContent />
    </AuthProvider>
  )
}