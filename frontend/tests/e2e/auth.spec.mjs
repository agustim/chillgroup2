// Test E2E de login i registre amb Playwright
// Aquest test verifica que el flux d'autenticació funciona correctament

import { test, expect } from '@playwright/test'

test.describe('ChillGroup Auth E2E', () => {
  test('página de login es mostra correctament', async ({ page }) => {
    await page.goto('/')
    
    await expect(page.locator('.login-header h1')).toContainText('ChillGroup v2')
    await expect(page.locator('.login-form')).toBeVisible()
    await expect(page.locator('#username')).toBeVisible()
    await expect(page.locator('#password')).toBeVisible()
    await expect(page.locator('.form-actions button')).toContainText('Entrar')
    await expect(page.locator('.toggle-auth')).toContainText('Registrar-se')
    
    // Password-hint no ha d'estar visible en mode login
    await expect(page.locator('.password-hint')).not.toBeVisible()
  })

  test('puc cambiar entre login i registre', async ({ page }) => {
    await page.goto('/')
    
    await expect(page.locator('.form-actions button')).toContainText('Entrar')
    
    await page.locator('.toggle-auth').click()
    
    await expect(page.locator('.form-actions button')).toContainText('Registrar-se')
    
    await page.locator('#password').fill('a')
    await expect(page.locator('.password-hint')).toBeVisible()
    await expect(page.locator('.password-hint')).toContainText('Mínim 8 caràcters')
    
    await page.locator('#password').fill('12345678')
    await expect(page.locator('.password-hint')).not.toBeVisible()
    
    await page.locator('.toggle-auth').click()
    
    await expect(page.locator('.form-actions button')).toContainText('Entrar')
    await expect(page.locator('.password-hint')).not.toBeVisible()
    
    await expect(page.locator('#username')).toHaveValue('')
    await expect(page.locator('#password')).toHaveValue('')
  })

  test('valida contrasenya curta en registre', async ({ page }) => {
    await page.goto('/')
    
    await page.locator('.toggle-auth').click()
    
    await page.locator('#username').fill('testuser')
    await page.locator('#password').fill('curta')
    
    await expect(page.locator('.password-hint')).toBeVisible()
    
    await page.locator('.form-actions button').click()
    
    // AuthContext pot mostrar error o no, però el hint sempre s'ha de mostrar
    await expect(page.locator('.password-hint')).toBeVisible()
  })

  test('omple el formulari correctament', async ({ page }) => {
    await page.goto('/')
    
    await page.locator('#username').fill('agusti')
    await page.locator('#password').fill('password123')
    
    await expect(page.locator('#username')).toHaveValue('agusti')
    await expect(page.locator('#password')).toHaveValue('password123')
    
    await page.locator('#username').clear()
    await expect(page.locator('#username')).toHaveValue('')
  })

  test('canvis entre login/registre neteja camps', async ({ page }) => {
    await page.goto('/')
    
    await page.locator('#username').fill('monusuari')
    await page.locator('#password').fill('password123')
    
    await page.locator('.toggle-auth').click()
    
    await expect(page.locator('#username')).toHaveValue('')
    await expect(page.locator('#password')).toHaveValue('')
    
    await page.locator('.toggle-auth').click()
    
    await expect(page.locator('#username')).toHaveValue('')
    await expect(page.locator('#password')).toHaveValue('')
  })
})