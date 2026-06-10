import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig(({ mode }) => {
  // Carrega variables d'entorn des del directori arrel del projecte
  const env = loadEnv(mode, path.resolve(__dirname, '..'), '')
  const livekitHost = env.LIVEKIT_HOST ?? env.VITE_LIVEKIT_URL ?? ''
  const openRegister = env.OPEN_REGISTER ?? env.VITE_OPEN_REGISTER ?? 'true'
  
  return {
    define: {
      __FRONTEND_DEBUG__: JSON.stringify(env.FRONTEND_DEBUG ?? env.VITE_FRONTEND_DEBUG ?? 'info'),
      __LIVEKIT_HOST__: JSON.stringify(livekitHost),
      __OPEN_REGISTER__: JSON.stringify(openRegister),
    },
    plugins: [react()],
    server: {
      host: '0.0.0.0',
      port: 5173,
      proxy: {
        '/api': {
          target: 'http://localhost:8080',
          changeOrigin: true,
        },
        '/health': {
          target: 'http://localhost:8080',
          changeOrigin: true,
        },
        '/socket.io': {
          target: 'http://localhost:8080',
          changeOrigin: true,
          ws: true,
        },
      },
    },
  }
})