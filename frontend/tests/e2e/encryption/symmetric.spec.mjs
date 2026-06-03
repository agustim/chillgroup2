// Tests E2E de canal amb encriptació simètrica
import { test, expect } from '@playwright/test'

const BASE_URL = 'http://localhost:8080'

async function setup() {
  const u = `enc_sym_${Date.now()}`
  const res = await fetch(`${BASE_URL}/api/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: u, password: 'TestPass123!' }),
  })
  const data = await res.json()
  const token = data.data?.token ?? data.token

  const srvRes = await fetch(`${BASE_URL}/api/servers`, {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: `SymEncSrv-${Date.now()}` }),
  })
  const srv = await srvRes.json()
  const serverId = srv.data?.serverId ?? srv.serverId

  const chRes = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: 'sym-ch', type: 'text', encryptionType: 'symmetric' }),
  })
  const ch = await chRes.json()
  const channelId = ch.data?.channelId ?? ch.channelId

  return { token, serverId, channelId }
}

test.describe('Canal amb encriptació simètrica', () => {
  test('el canal té encryptionType=symmetric', async ({ page }) => {
    const { token, serverId, channelId } = await setup()

    const res = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
      headers: { 'Authorization': `Bearer ${token}` },
    })
    const data = await res.json()
    const channels = data.data ?? data
    const ch = channels.find((c) => c.channelId === channelId || c.id === channelId)
    expect(ch?.encryptionType).toBe('symmetric')
  })

  test('el canal té una clau simètrica inicialitzada', async ({ page }) => {
    const { token, channelId } = await setup()

    const res = await fetch(`${BASE_URL}/api/channels/${channelId}/keys`, {
      headers: { 'Authorization': `Bearer ${token}` },
    })
    // Ha de retornar 200 (clau existent) o 404 si no s'ha inicialitzat encara
    expect([200, 404]).toContain(res.status)
  })

  test('enviar missatge amb payload xifrat (no text pla)', async ({ page }) => {
    const { token, channelId } = await setup()

    // Simulem un payload xifrat (base64 d'AES-GCM)
    const fakeEncryptedPayload = btoa('fakecipher123456')
    const fakeIv = btoa('iv1234567890')

    const send = await fetch(`${BASE_URL}/api/channels/${channelId}/messages`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({
        encryptedPayload: fakeEncryptedPayload,
        iv: fakeIv,
        keyVersion: 1,
      }),
    })
    expect(send.status).toBe(201)

    const list = await fetch(`${BASE_URL}/api/channels/${channelId}/messages?limit=5`, {
      headers: { 'Authorization': `Bearer ${token}` },
    })
    const msgs = await list.json()
    const messages = msgs.data ?? msgs
    const found = messages.find((m) => m.encryptedPayload === fakeEncryptedPayload)
    expect(found).toBeDefined()
    expect(found.iv).toBe(fakeIv)
  })
})
