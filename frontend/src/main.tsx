import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './i18n'
import './index.css'
import { initDesktopApiBase } from './lib/api'

initDesktopApiBase().then(() => {
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  )
})
