import { test, expect } from '@playwright/test'

test.describe('ChillGroup app UI', () => {
  test('login page renders expected fields', async ({ page }) => {
    await page.goto('/')

    await expect(page).toHaveTitle(/ChillGroup/)
    await expect(page.locator('.login-form')).toBeVisible()
    await expect(page.locator('#username')).toBeVisible()
    await expect(page.locator('#password')).toBeVisible()
    await expect(page.locator('.form-actions button')).toBeVisible()
  })

  test('username and password fields accept text', async ({ page }) => {
    await page.goto('/')

    await page.locator('#username').fill('testuser')
    await page.locator('#password').fill('password123')

    await expect(page.locator('#username')).toHaveValue('testuser')
    await expect(page.locator('#password')).toHaveValue('password123')
  })

  test('password input is masked', async ({ page }) => {
    await page.goto('/')

    await expect(page.locator('#password')).toHaveAttribute('type', 'password')
  })

  test('register mode can be toggled and submit label changes', async ({ page }) => {
    await page.goto('/')

    const submitButton = page.locator('.form-actions button')
    const toggle = page.locator('.toggle-auth')

    const initialLabel = await submitButton.innerText()
    await toggle.click()
    const toggledLabel = await submitButton.innerText()

    expect(toggledLabel).not.toBe(initialLabel)
  })

  test('switching auth mode clears form fields', async ({ page }) => {
    await page.goto('/')

    const username = page.locator('#username')
    const password = page.locator('#password')
    const toggle = page.locator('.toggle-auth')

    await username.fill('user-to-reset')
    await password.fill('password-to-reset')

    await toggle.click()
    await toggle.click()

    await expect(username).toHaveValue('')
    await expect(password).toHaveValue('')
  })

  test('empty submit stays on login view', async ({ page }) => {
    await page.goto('/')

    await page.locator('.form-actions button').click()

    await expect(page.locator('.login-form')).toBeVisible()
    await expect(page.locator('#username')).toBeVisible()
  })

  test('invalid credentials keep user on auth screen', async ({ page }) => {
    await page.goto('/')

    await page.locator('#username').fill('missing-user-xyz')
    await page.locator('#password').fill('wrong-password-xyz')
    await page.locator('.form-actions button').click()

    await expect(page.locator('.login-form')).toBeVisible()
  })
})
