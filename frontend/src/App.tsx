import React from 'react'
import { AuthProvider, useAuth } from './contexts/AuthContext'
import { LoginScreen } from './components/LoginScreen'
import { AppLayout } from './components/AppLayout'

function AppContent() {
  const { isAuthenticated, user, isLoading } = useAuth()

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
    return <AppLayout username={user.username} />
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