// Tests E2E de permisos de canal
import { test, expect } from '@playwright/test'

const BASE_URL = 'http://localhost:8080'

async function reg(suffix) {
  const u = `perm_${suffix}_${Date.now()}`
  const res = await fetch(`${BASE_URL}/api/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: u, password: 'TestPass123!' }),
  })
  const data = await res.json()
  return { username: u, token: data.data?.token ?? data.token, userId: data.data?.userId ?? data.userId }
}

test.describe('Permisos de canal - API', () => {
  test('owner pot crear un canal privat', async ({ page }) => {
    const owner = await reg('owner')
    const srvRes = await fetch(`${BASE_URL}/api/servers`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${owner.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: `PermServer-${Date.now()}` }),
    })
    const srv = await srvRes.json()
    const serverId = srv.data?.serverId ?? srv.serverId

    const chRes = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${owner.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'privat', type: 'text', encryptionType: 'none', isPrivate: true }),
    })
    expect(chRes.status).toBe(201)
    const ch = await chRes.json()
    const isPrivate = ch.data?.isPrivate ?? ch.isPrivate
    expect(isPrivate).toBe(true)
  })

  test('membre del servidor no pot eliminar canals (403)', async ({ page }) => {
    const owner = await reg('ch_owner')
    const member = await reg('ch_member')

    const srvRes = await fetch(`${BASE_URL}/api/servers`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${owner.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: `PermSrv2-${Date.now()}` }),
    })
    const srv = await srvRes.json()
    const serverId = srv.data?.serverId ?? srv.serverId

    // Afegir member al servidor
    await fetch(`${BASE_URL}/api/servers/${serverId}/members`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${owner.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: member.username }),
    })

    // Crear canal
    const chRes = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${owner.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'general', type: 'text', encryptionType: 'none' }),
    })
    const ch = await chRes.json()
    const channelId = ch.data?.channelId ?? ch.channelId

    // El membre intenta eliminar el canal
    const del = await fetch(`${BASE_URL}/api/servers/${serverId}/channels/${channelId}`, {
      method: 'DELETE',
      headers: { 'Authorization': `Bearer ${member.token}` },
    })
    expect([403, 401]).toContain(del.status)
  })

  test('usuari no autenticat no pot accedir als canals del servidor', async ({ page }) => {
    const owner = await reg('noauth_owner')
    const srvRes = await fetch(`${BASE_URL}/api/servers`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${owner.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: `NoAuthSrv-${Date.now()}` }),
    })
    const srv = await srvRes.json()
    const serverId = srv.data?.serverId ?? srv.serverId

    const res = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`)
    expect(res.status).toBe(401)
  })
})
