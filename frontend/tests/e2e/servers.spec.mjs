// Tests E2E de gestió de servidors (CRUD)
import { test, expect } from '@playwright/test'

const BASE_URL = 'http://localhost:8080'

async function registerAndLogin(page, username, password) {
  await page.goto('/')
  await page.locator('.toggle-auth').click()
  await page.locator('#username').fill(username)
  await page.locator('#password').fill(password)
  await page.locator('.form-actions button').click()
  await page.waitForURL(/\/app/)
}

async function apiRegisterAndLogin(username, password) {
  const reg = await fetch(`${BASE_URL}/api/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password }),
  })
  const data = await reg.json()
  return data.data?.token ?? data.token
}

test.describe('Servidors - CRUD', () => {
  test('la pàgina inicial mostra la llista de servidors buida', async ({ page }) => {
    const u = `srv_empty_${Date.now()}`
    await registerAndLogin(page, u, 'TestPass123!')
    await expect(page.locator('.server-bar')).toBeVisible()
  })

  test('crear un servidor mostra el nom al ServerBar', async ({ page }) => {
    const u = `srv_create_${Date.now()}`
    await registerAndLogin(page, u, 'TestPass123!')

    // Obrir modal de creació de servidor
    await page.locator('[data-testid="create-server-btn"], .create-server-btn, button[title*="servidor"], button[title*="Crear"], .add-server-btn').first().click()
    await page.locator('input[name="name"], input[placeholder*="nom"], input[placeholder*="name"]').first().fill('El Meu Servidor')
    await page.locator('button[type="submit"], .btn-primary').first().click()

    await expect(page.locator('.server-bar')).toContainText('El Meu Servidor', { timeout: 5000 })
  })

  test('seleccionar un servidor mostra la llista de canals', async ({ page }) => {
    const u = `srv_select_${Date.now()}`
    const token = await apiRegisterAndLogin(u, 'TestPass123!')

    // Crear servidor via API
    const srvRes = await fetch(`${BASE_URL}/api/servers`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Servidor Seleccio' }),
    })
    await srvRes.json()

    await registerAndLogin(page, u, 'TestPass123!')
    await page.locator('.server-item, [data-testid*="server"]').first().click()
    await expect(page.locator('.channel-list, .channels-sidebar')).toBeVisible({ timeout: 5000 })
  })
})
