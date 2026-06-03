// Tests E2E de gestió de canals
import { test, expect } from '@playwright/test'

const BASE_URL = 'http://localhost:8080'

async function setup(page) {
  const u = `ch_user_${Date.now()}`
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
    body: JSON.stringify({ name: `Server-${Date.now()}` }),
  })
  const srv = await srvRes.json()
  const serverId = srv.data?.serverId ?? srv.serverId

  // Login via UI
  await page.goto('/')
  await page.locator('.toggle-auth').click()
  await page.locator('#username').fill(u)
  await page.locator('#password').fill(pw)
  await page.locator('.form-actions button').click()
  await page.waitForURL(/\/app/)

  return { token, serverId, username: u }
}

test.describe('Canals - gestió', () => {
  test('la llista de canals es mostra al seleccionar servidor', async ({ page }) => {
    await setup(page)
    await page.locator('.server-item, [data-testid*="server"]').first().click()
    await expect(page.locator('.channel-list, .channels-sidebar, .channel-list-container')).toBeVisible({ timeout: 5000 })
  })

  test('API: crear canal de text retorna 201', async ({ page }) => {
    const { token, serverId } = await setup(page)

    const res = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'canal-test', type: 'text', encryptionType: 'none' }),
    })
    expect(res.status).toBe(201)
    const data = await res.json()
    expect(data.data?.name ?? data.name).toBe('canal-test')
  })

  test('API: crear canal de veu retorna 201', async ({ page }) => {
    const { token, serverId } = await setup(page)

    const res = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'veu-test', type: 'voice', encryptionType: 'none' }),
    })
    expect(res.status).toBe(201)
  })

  test('API: eliminar canal retorna 204', async ({ page }) => {
    const { token, serverId } = await setup(page)

    const create = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'to-delete', type: 'text', encryptionType: 'none' }),
    })
    const created = await create.json()
    const channelId = created.data?.channelId ?? created.channelId

    const del = await fetch(`${BASE_URL}/api/servers/${serverId}/channels/${channelId}`, {
      method: 'DELETE',
      headers: { 'Authorization': `Bearer ${token}` },
    })
    expect(del.status).toBe(204)
  })

  test('API: canal amb TTL configurat té el valor correcte', async ({ page }) => {
    const { token, serverId } = await setup(page)

    const res = await fetch(`${BASE_URL}/api/servers/${serverId}/channels`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'ttl-canal', type: 'text', encryptionType: 'none', messageTtl: 3600 }),
    })
    const data = await res.json()
    const ttl = data.data?.messageTtl ?? data.messageTtl
    expect(ttl).toBe(3600)
  })
})
