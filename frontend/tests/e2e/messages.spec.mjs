// Tests E2E de missatgeria
import { test, expect } from '@playwright/test'

const BASE_URL = 'http://localhost:8080'

async function apiSetup(suffix) {
  const u = `msg_${suffix}_${Date.now()}`
  const pw = 'TestPass123!'

  const reg = await fetch(`${BASE_URL}/api/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: u, password: pw }),
  })
  const data = await reg.json()
  const token = data.data?.token ?? data.token

  const srvRes = await fetch(`${BASE_URL}/api/servers`, {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: `MsgServer-${Date.now()}` }),
  })
  const srv = await srvRes.json()
  const serverId = srv.data?.serverId ?? srv.serverId

  const chRes = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: 'general', type: 'text', encryptionType: 'none' }),
  })
  const ch = await chRes.json()
  const channelId = ch.data?.channelId ?? ch.channelId

  return { token, serverId, channelId, username: u }
}

test.describe('Missatges - API', () => {
  test('enviar un missatge i recuperar-lo via API', async ({ page }) => {
    const { token, channelId } = await apiSetup('send')

    const sendRes = await fetch(`${BASE_URL}/api/channels/${channelId}/messages`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({
        encryptedPayload: 'Hola món',
        iv: '',
        keyVersion: null,
      }),
    })
    expect(sendRes.status).toBe(201)

    const listRes = await fetch(`${BASE_URL}/api/channels/${channelId}/messages?limit=10`, {
      headers: { 'Authorization': `Bearer ${token}` },
    })
    expect(listRes.status).toBe(200)
    const msgs = await listRes.json()
    const list = msgs.data ?? msgs
    expect(Array.isArray(list)).toBe(true)
    expect(list.length).toBeGreaterThan(0)
    const found = list.find((m) => m.encryptedPayload === 'Hola món')
    expect(found).toBeDefined()
  })

  test('eliminar un missatge retorna 204', async ({ page }) => {
    const { token, channelId } = await apiSetup('delete')

    const send = await fetch(`${BASE_URL}/api/channels/${channelId}/messages`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ encryptedPayload: 'per eliminar', iv: '', keyVersion: null }),
    })
    const sent = await send.json()
    const messageId = sent.data?.messageId ?? sent.messageId

    const del = await fetch(`${BASE_URL}/api/channels/${channelId}/messages/${messageId}`, {
      method: 'DELETE',
      headers: { 'Authorization': `Bearer ${token}` },
    })
    expect(del.status).toBe(204)
  })

  test('usuari sense accés no pot llegir missatges del canal', async ({ page }) => {
    const { channelId } = await apiSetup('priv')

    // Registrar un altre usuari sense accés al canal
    const reg2 = await fetch(`${BASE_URL}/api/auth/register`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: `outsider_${Date.now()}`, password: 'TestPass123!' }),
    })
    const d2 = await reg2.json()
    const token2 = d2.data?.token ?? d2.token

    const res = await fetch(`${BASE_URL}/api/channels/${channelId}/messages`, {
      headers: { 'Authorization': `Bearer ${token2}` },
    })
    expect([403, 404]).toContain(res.status)
  })

  test('llistar missatges sense autenticació retorna 401', async ({ page }) => {
    const { channelId } = await apiSetup('noauth')

    const res = await fetch(`${BASE_URL}/api/channels/${channelId}/messages`)
    expect(res.status).toBe(401)
  })
})
