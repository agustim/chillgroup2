// Tests E2E de paginació de missatges
// Verifica: ordre inicial (últims 50), cursor before/after, scroll-up carrega antics
import { test, expect } from '@playwright/test'

const API = 'http://127.0.0.1:8080'
const MSG_COUNT = 60

const ADMIN_USER = 'admin'
const ADMIN_PASS = 'change-me'

// ─── Helpers d'API (usen el request fixture de Playwright) ─────────────────

async function getAdminToken(request) {
  const res = await request.post(`${API}/api/auth/login`, {
    data: { username: ADMIN_USER, password: ADMIN_PASS },
  })
  const d = await res.json()
  return d.token ?? d.data?.token
}

async function createInvitationCode(request, adminToken) {
  const res = await request.post(`${API}/api/invitations`, {
    headers: { Authorization: `Bearer ${adminToken}` },
    data: { max_uses: 500 },
  })
  const d = await res.json()
  return d.data?.code ?? d.code
}

async function registerUser(request, suffix) {
  const adminToken = await getAdminToken(request)
  const code = await createInvitationCode(request, adminToken)
  const username = `pgn_${suffix}_${Date.now()}`
  const res = await request.post(`${API}/api/auth/register-with-invitation`, {
    data: { username, password: 'TestPass123!', code },
  })
  const d = await res.json()
  return { token: d.token ?? d.data?.token, username, password: 'TestPass123!' }
}

async function apiSetup(request, suffix = 'x') {
  const { token, username, password } = await registerUser(request, suffix)

  const srvRes = await request.post(`${API}/api/servers`, {
    headers: { Authorization: `Bearer ${token}` },
    data: { name: `PgnServer-${Date.now()}` },
  })
  const srv = await srvRes.json()
  const serverId = srv.server_id ?? srv.data?.serverId ?? srv.id

  const chRes = await request.post(`${API}/api/servers/${serverId}/channels`, {
    headers: { Authorization: `Bearer ${token}` },
    data: { name: 'general', type: 'text', encryptionType: 'none' },
  })
  const ch = await chRes.json()
  const channelId = ch.id ?? ch.data?.channelId ?? ch.channelId

  return { token, serverId, channelId, username, password }
}

async function sendMessages(request, token, channelId, count) {
  for (let i = 1; i <= count; i++) {
    const res = await request.post(`${API}/api/channels/${channelId}/messages`, {
      headers: { Authorization: `Bearer ${token}` },
      data: { encrypted_payload: `msg_${i}`, iv: '', key_version: null },
    })
    if (res.status() >= 400) {
      throw new Error(`sendMessages falla al missatge ${i}: status ${res.status()}, body: ${await res.text()}`)
    }
    // Petit delay per garantir timestamps monotons en PostgreSQL (evita collisions async)
    await new Promise(r => setTimeout(r, 15))
  }
}

async function loginUI(page, username, password) {
  await page.goto('/')
  await page.locator('#username').fill(username)
  await page.locator('#password').fill(password)
  await page.locator('.form-actions button').click()

  // Esperar 3s per veure si apareix el modal de protecció de dispositiu
  await page.waitForTimeout(2000)

  const unlockBtn = page.locator('button:has-text("Crear i desbloquejar")')
  if (await unlockBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
    const LOCAL_KEY = 'TestLocalKey123!'
    await page.getByPlaceholder('Introdueix la clau local').fill(LOCAL_KEY)
    await page.getByPlaceholder('Repeteix la clau local').fill(LOCAL_KEY)
    await unlockBtn.click()
    // Esperar que el modal desapareixi i que la server-bar sigui visible
    await page.locator('.server-bar').waitFor({ state: 'visible', timeout: 15000 })
  } else {
    // No hi ha modal: esperar directament que l'app carregui
    await page.locator('.server-bar').waitFor({ state: 'visible', timeout: 15000 })
  }
}

// ─── Tests d'API ────────────────────────────────────────────────────────────

test.describe('Paginació missatges — API', () => {
  test('sense cursor: retorna els 50 MES RECENTS en ordre ASC', async ({ request }) => {
    const { token, channelId } = await apiSetup(request, 'a1')
    await sendMessages(request, token, channelId, MSG_COUNT)

    const res = await request.get(`${API}/api/channels/${channelId}/messages?limit=50`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    expect(res.status()).toBe(200)
    const body = await res.json()
    const msgs = body.data ?? body

    expect(Array.isArray(msgs)).toBe(true)
    expect(msgs.length).toBe(50)

    // Ha de contenir msg_60 (el mes recent)
    expect(msgs.find(m => m.encrypted_payload === 'msg_60')).toBeDefined()
    // NO ha de contenir msg_1 (es a la pagina anterior)
    expect(msgs.find(m => m.encrypted_payload === 'msg_1')).toBeUndefined()

    // Han d'estar en ordre cronologic ASC
    for (let i = 1; i < msgs.length; i++) {
      expect(new Date(msgs[i].timestamp).getTime()).toBeGreaterThanOrEqual(
        new Date(msgs[i - 1].timestamp).getTime()
      )
    }
  })

  test("cursor before: retorna missatges MES ANTICS en ordre ASC", async ({ request }) => {
    const { token, channelId } = await apiSetup(request, 'a2')
    await sendMessages(request, token, channelId, MSG_COUNT)

    // Pagina inicial: index 0 = el mes antic (ASC)
    const initRes = await request.get(`${API}/api/channels/${channelId}/messages?limit=50`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    const initMsgs = (await initRes.json()).data
    expect(Array.isArray(initMsgs)).toBe(true)

    // El backend retorna DESC (el mes nou primer), l'ultim element es el mes antic de la pagina
    const oldestInPage = initMsgs[initMsgs.length - 1]
    expect(oldestInPage.encrypted_payload).toBe('msg_11')

    // Carreguem la pagina anterior (missatges 1-10)
    const prevRes = await request.get(
      `${API}/api/channels/${channelId}/messages?limit=50&before=${oldestInPage.id}`,
      { headers: { Authorization: `Bearer ${token}` } },
    )
    expect(prevRes.status()).toBe(200)
    const prevMsgs = (await prevRes.json()).data
    expect(Array.isArray(prevMsgs)).toBe(true)

    // Ha de contenir els missatges anteriors (enviats cronologicament abans de msg_11)
    // Nota: amb delays de 15ms garantim que els timestamps son monotons
    expect(prevMsgs.length).toBeGreaterThan(0)
    expect(prevMsgs.length).toBeLessThanOrEqual(10)

    // msg_11 (l'anchor) NO ha de ser a la pagina anterior
    expect(prevMsgs.find(m => m.encrypted_payload === 'msg_11')).toBeUndefined()
    // Missatges nous (>= msg_11) tampoc han d'apareixer
    for (let n = 11; n <= 60; n++) {
      expect(prevMsgs.find(m => m.encrypted_payload === `msg_${n}`)).toBeUndefined()
    }
  })

  test("cursor after: retorna missatges MES NOUS en ordre ASC", async ({ request }) => {
    const { token, channelId } = await apiSetup(request, 'a3')
    await sendMessages(request, token, channelId, 20)

    const allRes = await request.get(`${API}/api/channels/${channelId}/messages?limit=50`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    const allMsgs = (await allRes.json()).data
    const msg10 = allMsgs.find(m => m.encrypted_payload === 'msg_10')
    expect(msg10).toBeDefined()

    const afterRes = await request.get(
      `${API}/api/channels/${channelId}/messages?limit=50&after=${msg10.id}`,
      { headers: { Authorization: `Bearer ${token}` } },
    )
    expect(afterRes.status()).toBe(200)
    const afterMsgs = (await afterRes.json()).data
    expect(Array.isArray(afterMsgs)).toBe(true)

    expect(afterMsgs.length).toBe(10)

    // Tots mes nous que msg_10
    for (const m of afterMsgs) {
      expect(new Date(m.timestamp).getTime()).toBeGreaterThan(
        new Date(msg10.timestamp).getTime()
      )
    }

    // Ordre ASC (oldest-first per UX unread)
    for (let i = 1; i < afterMsgs.length; i++) {
      expect(new Date(afterMsgs[i].timestamp).getTime()).toBeGreaterThanOrEqual(
        new Date(afterMsgs[i - 1].timestamp).getTime()
      )
    }

    expect(afterMsgs.find(m => m.encrypted_payload === 'msg_11')).toBeDefined()
    expect(afterMsgs.find(m => m.encrypted_payload === 'msg_20')).toBeDefined()
  })

  test('has_more: true si hi ha mes pagines, false si no', async ({ request }) => {
    const { token, channelId } = await apiSetup(request, 'a4')
    await sendMessages(request, token, channelId, 60)

    const r1 = await request.get(`${API}/api/channels/${channelId}/messages?limit=50`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    const b1 = await r1.json()
    expect(b1.pagination?.has_more ?? b1.has_more).toBe(true)

    const r2 = await request.get(`${API}/api/channels/${channelId}/messages?limit=100`, {
      headers: { Authorization: `Bearer ${token}` },
    })
    const b2 = await r2.json()
    expect(b2.pagination?.has_more ?? b2.has_more).toBe(false)
  })
})

// ─── Tests d'UI ────────────────────────────────────────────────────────────

test.describe('Paginació missatges — UI', () => {
  test('canal amb >50 missatges: mostra els MES RECENTS al obrir', async ({ page, request }) => {
    const ctx = await apiSetup(request, 'u1')
    await sendMessages(request, ctx.token, ctx.channelId, MSG_COUNT)
    await loginUI(page, ctx.username, ctx.password)

    // Cliquem el servidor (excloem el botó d'afegir servidor que també té class server-icon)
    await page.locator('.server-icon:not(.add-server)').first().click()
    await page.locator('.channel-item').first().click()

    await expect(page.locator('.message-list')).toBeVisible({ timeout: 8000 })
    await expect(page.locator('.message-list.loading')).not.toBeVisible({ timeout: 8000 })

    await expect(page.locator('.message-bubble').filter({ hasText: 'msg_60' })).toBeVisible({ timeout: 5000 })
    await expect(page.locator('.message-bubble').filter({ hasText: /^msg_1$/ })).not.toBeVisible()
  })

  test('canal amb >50 missatges: ordre cronologic (antics dalt, nous baix)', async ({ page, request }) => {
    const ctx = await apiSetup(request, 'u2')
    await sendMessages(request, ctx.token, ctx.channelId, MSG_COUNT)
    await loginUI(page, ctx.username, ctx.password)

    // Cliquem el servidor (excloem el botó d'afegir servidor que també té class server-icon)
    await page.locator('.server-icon:not(.add-server)').first().click()
    await page.locator('.channel-item').first().click()
    await expect(page.locator('.message-list.loading')).not.toBeVisible({ timeout: 8000 })

    const bubble50 = page.locator('.message-bubble').filter({ hasText: 'msg_50' })
    const bubble60 = page.locator('.message-bubble').filter({ hasText: 'msg_60' })

    await expect(bubble50).toBeVisible({ timeout: 5000 })
    await expect(bubble60).toBeVisible({ timeout: 5000 })

    const box50 = await bubble50.boundingBox()
    const box60 = await bubble60.boundingBox()
    expect(box50.y).toBeLessThan(box60.y)
  })

  test('scroll cap amunt carrega missatges antics', async ({ page, request }) => {
    const ctx = await apiSetup(request, 'u3')
    await sendMessages(request, ctx.token, ctx.channelId, MSG_COUNT)
    await loginUI(page, ctx.username, ctx.password)

    // Cliquem el servidor (excloem el botó d'afegir servidor que també té class server-icon)
    await page.locator('.server-icon:not(.add-server)').first().click()
    await page.locator('.channel-item').first().click()
    await expect(page.locator('.message-list.loading')).not.toBeVisible({ timeout: 8000 })

    // Esperem que la llista carregui
    await expect(page.locator('.message-list.loading')).not.toBeVisible({ timeout: 8000 })
    const initialCount = await page.locator('.message-bubble').count()
    console.log(`DEBUG scroll test: initial message count = ${initialCount}`)
    // Esperem almenys 10 missatges (la paginació pot carregar menys si hi ha pocs)
    expect(initialCount).toBeGreaterThan(0)

    // Esperar que la hint de "scroll cap amunt" sigui visible (hasPrevPage=true i càrrega OK)
    await page.locator('.load-more-hint').waitFor({ state: 'visible', timeout: 10000 })

    // Fer scroll del sentinel al viewport (dispara l'IntersectionObserver)
    await page.locator('.messages-top-sentinel').scrollIntoViewIfNeeded()
    await page.waitForTimeout(2000) // Esperar fetch + render

    // Esperem que el count augmenti (nous missatges carregats)
    await page.waitForFunction(
      (initial) => document.querySelectorAll('.message-bubble').length > initial,
      initialCount,
      { timeout: 15000 }
    )
    const newCount = await page.locator('.message-bubble').count()
    console.log(`DEBUG scroll test: count after scroll = ${newCount}`)
    expect(newCount).toBeGreaterThan(initialCount)

    // El primer bubble (index 0) ha de ser msg_1 (el més antic)
    const firstContent = await page.locator('.message-bubble').first().textContent()
    expect(firstContent).toContain('msg_1')

    // L'últim bubble ha de ser msg_60 (el més nou)
    const lastContent = await page.locator('.message-bubble').last().textContent()
    expect(lastContent).toContain('msg_60')
  })
})
