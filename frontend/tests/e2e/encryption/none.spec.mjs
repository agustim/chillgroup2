// Tests E2E de canal sense encriptació
import { test, expect } from '@playwright/test'

const BASE_URL = 'http://localhost:8080'

async function setup() {
  const u = `enc_none_${Date.now()}`
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
    body: JSON.stringify({ name: `NoneEncSrv-${Date.now()}` }),
  })
  const srv = await srvRes.json()
  const serverId = srv.data?.serverId ?? srv.serverId

  const chRes = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: 'public-ch', type: 'text', encryptionType: 'none' }),
  })
  const ch = await chRes.json()
  const channelId = ch.data?.channelId ?? ch.channelId

  return { token, serverId, channelId }
}

test.describe('Canal sense encriptació (none)', () => {
  test('el canal té encryptionType=none', async ({ page }) => {
    const { token, serverId, channelId } = await setup()

    const res = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
      headers: { 'Authorization': `Bearer ${token}` },
    })
    const data = await res.json()
    const channels = data.data ?? data
    const ch = channels.find((c) => c.channelId === channelId || c.id === channelId)
    expect(ch?.encryptionType).toBe('none')
  })

  test('enviar i recuperar missatge en text pla (payload = text original)', async ({ page }) => {
    const { token, channelId } = await setup()
    const plaintext = 'Missatge sense xifrar'

    await fetch(`${BASE_URL}/api/channels/${channelId}/messages`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ encryptedPayload: plaintext, iv: '', keyVersion: null }),
    })

    const list = await fetch(`${BASE_URL}/api/channels/${channelId}/messages?limit=5`, {
      headers: { 'Authorization': `Bearer ${token}` },
    })
    const msgs = await list.json()
    const messages = msgs.data ?? msgs
    const found = messages.find((m) => m.encryptedPayload === plaintext)
    expect(found).toBeDefined()
  })
})
