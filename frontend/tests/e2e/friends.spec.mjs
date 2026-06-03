// Tests E2E de gestió d'amics
import { test, expect } from '@playwright/test'

const BASE_URL = 'http://localhost:8080'

async function reg(suffix) {
  const u = `friend_${suffix}_${Date.now()}`
  const res = await fetch(`${BASE_URL}/api/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: u, password: 'TestPass123!' }),
  })
  const data = await res.json()
  return { username: u, token: data.data?.token ?? data.token, userId: data.data?.userId ?? data.userId }
}

test.describe('Amics - API', () => {
  test('afegir amic i verificar que apareix a la llista', async ({ page }) => {
    const u1 = await reg('a')
    const u2 = await reg('b')

    const add = await fetch(`${BASE_URL}/api/friends`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${u1.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: u2.username }),
    })
    expect([200, 201]).toContain(add.status)

    const list = await fetch(`${BASE_URL}/api/friends`, {
      headers: { 'Authorization': `Bearer ${u1.token}` },
    })
    expect(list.status).toBe(200)
    const data = await list.json()
    const friends = data.data ?? data
    const found = friends.find((f) => f.username === u2.username || f.friendUsername === u2.username)
    expect(found).toBeDefined()
  })

  test('eliminar amic i verificar que no apareix', async ({ page }) => {
    const u1 = await reg('del_a')
    const u2 = await reg('del_b')

    await fetch(`${BASE_URL}/api/friends`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${u1.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: u2.username }),
    })

    const del = await fetch(`${BASE_URL}/api/friends/${u2.userId ?? u2.username}`, {
      method: 'DELETE',
      headers: { 'Authorization': `Bearer ${u1.token}` },
    })
    expect([200, 204]).toContain(del.status)

    const list = await fetch(`${BASE_URL}/api/friends`, {
      headers: { 'Authorization': `Bearer ${u1.token}` },
    })
    const data = await list.json()
    const friends = data.data ?? data
    const found = friends.find((f) => f.username === u2.username || f.friendUsername === u2.username)
    expect(found).toBeUndefined()
  })

  test('afegir el mateix amic dues vegades no falla (idempotent o retorna error clar)', async ({ page }) => {
    const u1 = await reg('dup_a')
    const u2 = await reg('dup_b')

    await fetch(`${BASE_URL}/api/friends`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${u1.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: u2.username }),
    })
    const dup = await fetch(`${BASE_URL}/api/friends`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${u1.token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ username: u2.username }),
    })
    // Ha de ser OK (idempotent) o 409 (conflict clar)
    expect([200, 201, 409]).toContain(dup.status)
  })
})
