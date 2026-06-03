// Tests E2E de canal amb encriptació asimètrica (E2EE)
import { test, expect } from '@playwright/test'

const BASE_URL = 'http://localhost:8080'

async function reg(suffix) {
  const u = `e2ee_${suffix}_${Date.now()}`
  const res = await fetch(`${BASE_URL}/api/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: u, password: 'TestPass123!' }),
  })
  const data = await res.json()
  return { username: u, token: data.data?.token ?? data.token, userId: data.data?.userId ?? data.userId }
}

test.describe('Canal asimètric E2EE', () => {
  test('el canal té encryptionType=asymmetric', async ({ page }) => {
    const owner = await reg('creator')
    const srvRes = await fetch(`${BASE_URL}/api/servers`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${owner.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: `AsymSrv-${Date.now()}` }),
    })
    const srv = await srvRes.json()
    const serverId = srv.data?.serverId ?? srv.serverId

    const chRes = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${owner.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'e2ee-ch', type: 'text', encryptionType: 'asymmetric' }),
    })
    expect(chRes.status).toBe(201)
    const ch = await chRes.json()
    expect(ch.data?.encryptionType ?? ch.encryptionType).toBe('asymmetric')
  })

  test("usuari no autoritzat no pot llegir missatges d'un canal asimètric", async ({ page }) => {
    const owner = await reg('asym_owner')
    const outsider = await reg('asym_outsider')

    const srvRes = await fetch(`${BASE_URL}/api/servers`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${owner.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: `AsymSrv2-${Date.now()}` }),
    })
    const srv = await srvRes.json()
    const serverId = srv.data?.serverId ?? srv.serverId

    const chRes = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${owner.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'secret', type: 'text', encryptionType: 'asymmetric' }),
    })
    const ch = await chRes.json()
    const channelId = ch.data?.channelId ?? ch.channelId

    // Outsider no és membre del servidor
    const res = await fetch(`${BASE_URL}/api/channels/${channelId}/messages`, {
      headers: { 'Authorization': `Bearer ${outsider.token}` },
    })
    expect([403, 404]).toContain(res.status)
  })

  test('el servidor emmagatzema el payload xifrat, no el text pla', async ({ page }) => {
    const owner = await reg('asym_store')
    const srvRes = await fetch(`${BASE_URL}/api/servers`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${owner.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: `AsymSrv3-${Date.now()}` }),
    })
    const srv = await srvRes.json()
    const serverId = srv.data?.serverId ?? srv.serverId

    const chRes = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${owner.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'enc-store', type: 'text', encryptionType: 'asymmetric' }),
    })
    const ch = await chRes.json()
    const channelId = ch.data?.channelId ?? ch.channelId

    // Enviar payload suposadament xifrat (base64)
    const encPayload = btoa('ciphertext_not_plaintext_xyz_123')
    const iv = btoa('randomiv12345678')
    await fetch(`${BASE_URL}/api/channels/${channelId}/messages`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${owner.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ encryptedPayload: encPayload, iv, keyVersion: 1 }),
    })

    const listRes = await fetch(`${BASE_URL}/api/channels/${channelId}/messages?limit=5`, {
      headers: { 'Authorization': `Bearer ${owner.token}` },
    })
    const data = await listRes.json()
    const messages = data.data ?? data
    const msg = messages[0]
    // El servidor guarda el payload tal com el rep (xifrat)
    expect(msg?.encryptedPayload).toBe(encPayload)
    expect(msg?.encryptedPayload).not.toBe('ciphertext_not_plaintext_xyz_123')
  })
})
