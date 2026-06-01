# ChillGroup v2 — Estratègia de Tests i Proves E2E

## Filosofía: TDD First

**Cada funcionalitat es codifica després de definir el seu test.**
No es commit cap feature sense el seu test corresponent.

```
1. Escrivim el test (FAIL)
2. Escrivim el codi mínim per fer-lo passar (PASS)
3. Refactoritzem (clean up)
4. Repetim
```

## Estructura de Tests

```
chillgroup/
├── tests/                              # Tests E2E Playwright (globals)
│   ├── README.md                       # Guia d'execució
│   ├── playwright.config.ts            # Config Playwright
│   ├── fixtures/                       # Fixtures personalitzats
│   │   ├── auth.fixture.ts             # Autenticació reutilitzable
│   │   ├── server.fixture.ts           # Servidor test
│   │   └── channel.fixture.ts          # Canals de test
│   ├── e2e/                            # Tests E2E complets
│   │   ├── auth.spec.ts                # Registre i login
│   │   ├── servers.spec.ts             # CRUD servidors
│   │   ├── channels.spec.ts            # Crear canals, TTL
│   │   ├── messages.spec.ts            # Enviar, editar, eliminar
│   │   ├── voice.spec.ts               # Connexió veu, mic, camera
│   │   ├── encryption/                 # Tests criptografia
│   │   │   ├── none-encryption.spec.ts # Canal sense encriptació
│   │   │   ├── symmetric.spec.ts       # Clau simètrica
│   │   │   └── asymmetric.spec.ts      # Clau asimètrica (E2EE)
│   │   └── voice-e2ee.spec.ts          # E2EE de veu (LiveKit)
│   ├── helpers/                        # Helpers de test
│   │   ├── crypto-helpers.ts           # Generar claus test
│   │   ├── channel-helpers.ts          # Crear canals test
│   │   └── user-helpers.ts             # Crear usuaris test
│   └── utils/                          # Utilitats
│       ├── test-db.ts                  # BD de test aïllada
│       ├── cleanup.ts                  # Neteja post-test
│       └── mock-livekit.ts             # Mock LiveKit
│
├── server/
│   └── tests/                          # Tests unitaris i d'integració Rust
│       ├── Cargo.toml                  # [dev-dependencies] = tokio-test, axum-test
│       ├── unit/
│       │   ├── crypto/
│       │   │   ├── kyber_test.rs       # Generar claus, encapsular, desencapsular
│       │   │   ├── aes_gcm_test.rs     # Encrypt/decrypt missatges
│       │   │   └── channel_keys_test.rs # Rotació, distribució
│       │   ├── auth_test.rs            # Hash passwords, JWT
│       │   └── message_test.rs         # Crear, filtrar, TTL
│       ├── integration/
│       │   ├── auth_flow_test.rs       # Registre → login → accedir
│       │   ├── channel_flow_test.rs    # Crear → convidar → accedir
│       │   ├── message_flow_test.rs    # Enviar → rebre → historial
│       │   └── crypto_flow_test.rs     # E2EE complet: crear canal → convidar → xifrar → desxifrar
│       └── fixtures/
│           └── test_data.rs            # Dades de test reutilitzables
│
├── frontend/
│   └── tests/
│       ├── unit/                       # Tests unitaris React (Vitest)
│       │   ├── crypto.test.ts          # Encrypt/decrypt, Kyber KEM
│       │   ├── api.test.ts             # Client API wrapper
│       │   ├── storage.test.ts         # IndexedDB operations
│       │   ├── local-vault.test.ts     # Setup/unlock/rotate de clau local
│       │   └── hooks/
│       │       ├── useAuth.test.tsx
│       │       ├── useChannelKey.test.ts
│       │       └── useMessages.test.tsx
│       ├── e2e/                        # Playwright E2E frontend
│       │   ├── login.spec.ts           # Login/registre
│       │   ├── channel-encryption.spec.ts # Crear canal → enviar missatge xifrat
│       │   └── voice-e2ee.spec.ts      # Connexió veu + E2EE
│       └── playwright.config.ts        # Config Playwright frontend
```

## Playwright E2E — Configuració

```typescript
// tests/playwright.config.ts
import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  timeout: 60_000,        // 60s per test (E2EE té cost computacional)
  fullyParallel: true,    // Executar tests en paral·lel
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined, // Serial en CI per BD
  reporter: [
    ['html', { open: 'never' }],
    ['json', { outputFile: 'test-results/results.json' }],
    ['list'],
  ],
  use: {
    baseURL: 'http://localhost:5173',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    trace: 'retain-on-failure',
  },

  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
    },
  ],

  webServer: [
    {
      command: 'cd server && cargo run',
      port: 8080,
      reuseExistingServer: !process.env.CI,
    },
    {
      command: 'cd frontend && npm run dev',
      port: 5173,
      reuseExistingServer: !process.env.CI,
    },
  ],

  // Config de test per a E2EE
  testMatch: /.*\.spec\.ts$/,
})
```

## Fixtures Personalitzats

### Auth Fixture

```typescript
// tests/fixtures/auth.fixture.ts
import { test as base } from '@playwright/test'
import { v4 as uuidv4 } from 'uuid'

export interface TestUser {
  username: string
  password: string
  deviceId: string
  publicKey: string
  keypair: Uint8Array // Kyber secret key (només per a test)
}

export const test = base.extend<{
  user1: TestUser
  user2: TestUser
  authenticatedPage: (user: TestUser) => Promise<void>
}>({
  user1: async ({}, use) => {
    const username = `user_${uuidv4().slice(0, 8)}`
    const password = 'TestPass123!'

    // Registrar usuari
    const resp = await fetch('http://localhost:8080/api/auth/register', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username, password }),
    })
    const data = await resp.json()

    // Generar keypair Kyber-1024 per al test
    const { publicKey, secretKey } = await cryptoSubtleGenKyberKey()

    // Guardar publicKey al servidor
    await fetch(`http://localhost:8080/api/user/me/devices/test-device/publicKey`, {
      method: 'PUT',
      headers: {
        'Authorization': `Bearer ${data.token}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ publicKey: btoa(String.fromCharCode(...publicKey)) }),
    })

    await use({
      username,
      password,
      deviceId: 'test-device',
      publicKey: btoa(String.fromCharCode(...publicKey)),
      keypair: secretKey,
    })

    // Netegar usuari després del test
    await fetch(`http://localhost:8080/api/auth/users/${username}`, {
      method: 'DELETE',
      headers: { 'Authorization': `Bearer ${data.token}` },
    })
  },

  user2: ['user1', async ({}, use) => {
    // Idèntic a user1 però amb nom diferent
    const username = `user_${uuidv4().slice(0, 8)}`
    const password = 'TestPass123!'

    const resp = await fetch('http://localhost:8080/api/auth/register', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username, password }),
    })
    const data = await resp.json()

    const { publicKey } = await cryptoSubtleGenKyberKey()

    await use({
      username,
      password,
      deviceId: 'test-device-2',
      publicKey: btoa(String.fromCharCode(...publicKey)),
      keypair: new Uint8Array(0), // No necessària en tests d'usuari 2
    })

    await fetch(`http://localhost:8080/api/auth/users/${username}`, {
      method: 'DELETE',
      headers: { 'Authorization': `Bearer ${data.token}` },
    })
  }, { inherits: ['user1'] }],

  authenticatedPage: async ({ page }, use) => {
    await use(async (user: TestUser) => {
      // Navegar a login
      await page.goto('/login')
      await page.fill('input[name="username"]', user.username)
      await page.fill('input[name="password"]', user.password)
      await page.click('button[type="submit"]')
      await page.waitForURL('/app/**')
    })
  },
})
```

### Server Fixture

```typescript
// tests/fixtures/server.fixture.ts
import { test as base } from './auth.fixture'

export const test = base.extend<{
  serverId: string
  serverName: string
  createServer: (name: string) => Promise<string>
}>({
  serverId: '',
  serverName: '',

  createServer: async ({ page, user1 }, use) => {
    let lastServerId: string = ''

    await use(async (name: string) => {
      const resp = await fetch('http://localhost:8080/api/servers', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${user1.token}`, // Necessari al fixture
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ name }),
      })
      const data = await resp.json()
      lastServerId = data.id
      return data.id
    })

    // Netegar servidor
    if (lastServerId) {
      await fetch(`http://localhost:8080/api/servers/${lastServerId}`, {
        method: 'DELETE',
        headers: { 'Authorization': `Bearer ${user1.token}` },
      })
    }
  },
})
```

## Scripts d'Execució

### package.json (root)

```json
{
  "name": "chillgroup-tests",
  "private": true,
  "scripts": {
    "test": "playwright test",
    "test:ui": "playwright test --ui",
    "test:headed": "playwright test --headed",
    "test:e2e": "playwright test e2e/",
    "test:e2e:encryption": "playwright test e2e/encryption/",
    "test:e2e:voice-e2ee": "playwright test e2e/voice-e2ee.spec.ts",
    "test:e2e:asymmetric": "playwright test e2e/encryption/asymmetric.spec.ts",
    "test:single": "playwright test --grep",
    "test:report": "playwright show-report",
    "test:coverage": "c8 --reporter=html playwright test",
    "test:ci": "playwright test --reporter=github",
    "test:clean": "playwright test --grep \"^$\" && rm -rf test-results/ test-report.html",
    "test:watch": "playwright test --watch"
  }
}
```

### Makefile

```makefile
.PHONY: test test-e2e test-encryption test-voice-e2ee test-server test-frontend clean

# Executar tots els tests
test:
	npm test

# Tests E2E complets (frontend + backend)
test-e2e:
	npm run test:e2e

# Tests d'encriptació específics
test-encryption:
	npm run test:e2e:encryption

# Test E2EE de veu
test-voice-e2ee:
	npm run test:e2e:voice-e2ee

# Test asimètric (E2EE complet)
test-asymmetric:
	npm run test:e2e:asymmetric

# Tests unitaris Rust
test-server:
	cd server && cargo test

# Tests unitaris frontend
test-frontend:
	cd frontend && npm test

# Executar tot en mode CI
test-ci:
	npm run test:ci

# Netejar resultats
clean:
	rm -rf test-results/ test-report.html playwright-report/
	cd server && cargo clean 2>/dev/null || true

# Execució rápida per desenvolupament (només un fitxer)
dev-test:
	npm run test:headed -- tests/e2e/$(FILE)
```

## Tests E2E — Escenaris Detallats

### 1. auth.spec.ts — Registre i Login

```typescript
// tests/e2e/auth.spec.ts
import { test, expect } from '@playwright/test'

test.describe('Autenticació', () => {
  test('usuari es pot registrar', async ({ page }) => {
    await page.goto('/login')

    await page.click('text=Registra\'t')
    await page.fill('input[name="username"]', 'testuser1')
    await page.fill('input[name="password"]', 'Password123!')
    await page.click('button[type="submit"]')

    // Esperar redirecció al dashboard
    await page.waitForURL('/app/**')
    await expect(page.locator('.server-bar')).toBeVisible()

    // Verificar que apareix el username
    await expect(page.locator('.user-info')).toContainText('testuser1')
  })

  test('usuari existent no es pot registrar dues vegades', async ({ page }) => {
    await page.goto('/login')

    // Registrar primer cop
    await page.click('text=Registra\'t')
    await page.fill('input[name="username"]', 'duplicateuser')
    await page.fill('input[name="password"]', 'Password123!')
    await page.click('button[type="submit"]')
    await page.waitForURL('/app/**')

    // Navegar a login de nou
    await page.goto('/login')

    // Intentar registrar amb el mateix username
    await page.click('text=Registra\'t')
    await page.fill('input[name="username"]', 'duplicateuser')
    await page.fill('input[name="password"]', 'Password456!')
    await page.click('button[type="submit"]')

    // Hauria de mostrar error
    await expect(page.locator('.error-message')).toBeVisible()
    await expect(page.locator('.error-message')).toContainText('ja existeix')
  })

  test('login amb credencials incorrectes mostra error', async ({ page }) => {
    await page.goto('/login')

    await page.fill('input[name="username"]', 'testuser1')
    await page.fill('input[name="password"]', 'wrongpassword')
    await page.click('button[type="submit"]')

    await expect(page.locator('.error-message')).toBeVisible()
  })

  test('després del login es genera keypair Kyber', async ({ page }) => {
    await page.goto('/login')

    await page.fill('input[name="username"]', 'testuser1')
    await page.fill('input[name="password"]', 'Password123!')
    await page.click('button[type="submit"]')
    await page.waitForURL('/app/**')

    // Verificar que el keypair es va guardar a IndexedDB
    const keypairExists = await page.evaluate(async () => {
      return new Promise((resolve) => {
        const request = indexedDB.open('chillgroup-store', 1)
        request.onsuccess = () => {
          const db = request.result
          const tx = db.transaction('keypairs', 'readonly')
          const store = tx.objectStore('keypairs')
          const getAll = store.getAll()
          getAll.onsuccess = () => {
            resolve(getAll.result.length > 0)
          }
        }
      })
    })

    expect(keypairExists).toBe(true)
  })
})
```

### 2. messages.spec.ts — Enviar Missatges

```typescript
// tests/e2e/messages.spec.ts
import { test, expect } from '@playwright/test'

test.describe('Missatges', () => {
  test.beforeEach(async ({ page, user1, createServer }) => {
    // Login
    await page.goto('/login')
    await page.fill('input[name="username"]', user1.username)
    await page.fill('input[name="password"]', user1.password)
    await page.click('button[type="submit"]')
    await page.waitForURL('/app/**')

    // Crear servidor i canal
    const serverId = await createServer('Test Server')
    // ... crear canal via API
  })

  test('usuari pot enviar missatge a canal de text', async ({ page }) => {
    // Seleccionar canal de text
    await page.click('.channel-item.text-channel')

    // Escriure missatge
    await page.fill('.message-input', 'Aquest és un missatge de prova')
    await page.press('.message-input', 'Enter')

    // Verificar que el missatge apareix
    await expect(page.locator('.message-bubble')).toContainText('Aquest és un missatge de prova')
  })

  test('els missatges expiren segons TTL', async ({ page }) => {
    // Canal amb TTL de 60s (via API)
    // ...

    // Esperar que expiri
    await page.waitForTimeout(65_000)

    // Verificar que el missatge ha desaparegut
    await expect(page.locator('.message-bubble')).not.toBeVisible()
  })
})
```

### 3. encryption/none-encryption.spec.ts — Sense Encriptació

```typescript
// tests/e2e/encryption/none-encryption.spec.ts
import { test, expect } from '@playwright/test'

test.describe('Canal sense encriptació', () => {
  test('els missatges es poden llegir en text pla', async ({ page }) => {
    await page.goto('/login')
    await page.fill('input[name="username"]', 'user_1')
    await page.fill('input[name="password"]', 'Password123!')
    await page.click('button[type="submit"]')
    await page.waitForURL('/app/**')

    // Crear canal sense encriptació (per defecte)
    await page.click('.new-channel-btn')
    await page.fill('input[name="channel-name"]', 'canal-public')
    await page.selectOption('select[name="encryption"]', 'none')
    await page.click('button[type="submit"]')

    // Enviar missatge
    await page.click('.channel-item:text("canal-public")')
    await page.fill('.message-input', 'Missatge en clar')
    await page.press('.message-input', 'Enter')

    // Verificar que es veu en text pla
    await expect(page.locator('.message-bubble')).toContainText('Missatge en clar')

    // Obtenir missatge via API i verificar que està en text pla a la resposta
    const response = await page.evaluate(async () => {
      const res = await fetch('/api/channels/test-channel/messages')
      const data = await res.json()
      return data[0]?.encryptedPayload
    })

    // En un canal sense encriptació, el payload és el text literal
    expect(response).toBe('Missatge en clar')
  })
})
```

### 4. encryption/symmetric.spec.ts — Clau Simètrica

```typescript
// tests/e2e/encryption/symmetric.spec.ts
import { test, expect } from '@playwright/test'

test.describe('Canal amb clau simètrica', () => {
  test('usuaris poden comunicar-se en un canal simètric', async ({ page }) => {
    await page.goto('/login')
    await page.fill('input[name="username"]', 'user_creator')
    await page.fill('input[name="password"]', 'Password123!')
    await page.click('button[type="submit"]')
    await page.waitForURL('/app/**')

    // 1. Crear canal simètric
    await page.click('.new-channel-btn')
    await page.fill('input[name="channel-name"]', 'canal-simetric')
    await page.selectOption('select[name="encryption"]', 'symmetric')
    await page.click('button[type="submit"]')
    await page.waitForTimeout(1000) // Esperar que es generi la clau

    // 2. Verificar que el canal té icona de clau
    await expect(page.locator('.channel-item .encryption-icon')).toContainText('🔑')

    // 3. Enviar missatge xifrat
    const testMessage = 'Missatge secret simètric'
    await page.fill('.message-input', testMessage)
    await page.press('.message-input', 'Enter')

    // 4. Verificar que el missatge es mostra correctament (desencriptat pel client)
    await expect(page.locator('.message-bubble')).toContainText(testMessage)

    // 5. Verificar que el payload a la DB està xifrat
    const encryptedPayload = await page.evaluate(async () => {
      const res = await fetch('/api/channels/symmetric-channel/messages')
      const data = await res.json()
      return data[0]?.encryptedPayload
    })

    // El payload xifrat NO ha de ser el text pla
    expect(encryptedPayload).not.toBe(testMessage)
    // Ha de ser base64 (longitud > 50 per un missatge tan curt amb AES-GCM)
    expect(encryptedPayload.length).toBeGreaterThan(50)
  })
})
```

### 5. encryption/asymmetric.spec.ts — Clau Asimètrica (E2EE) ⭐

**Aquest és el test crític.** Verifica el flux complet E2EE:

```typescript
// tests/e2e/encryption/asymmetric.spec.ts
import { test, expect } from '@playwright/test'

test.describe('Canal asimètric E2EE (Zero Knowledge)', () => {
  test('creator pot enviar i receptor pot desxifrar missatges', async ({ page }) => {
    // === USUARI 1: CREADOR ===
    // Registrar i login creator
    await registerAndLogin(page, 'creator_e2ee', 'Password123!')
    await page.waitForURL('/app/**')

    // Verificar que s'ha generat el keypair Kyber
    await expectKyberKeypairExists(page)

    // Crear canal asimètric
    await createChannel(page, 'canal-secrt', 'asymmetric')
    await expect(page.locator('.encryption-badge')).toContainText('🔒')

    // === USUARI 2: RECEPTOR (nova finestra/incognito) ===
    const context2 = await page.browserContext().storageState()

    // Obrir segona finestra amb sessió independent (simula altre dispositiu)
    const contextB = await page.browserContext().newContext()
    const page2 = await contextB.newPage()

    await registerAndLogin(page2, 'receptor_e2ee', 'Password123!')
    await page2.waitForURL('/app/**')

    // Verificar que el receptor veu el canal (perquè té accés)
    await expect(page2.locator('.channel-item:text("canal-secrt")')).toBeVisible()

    // === USUARI 1: Convida usuari 2 ===
    await page.click('.invite-btn')
    await page.fill('input[name="username"]', 'receptor_e2ee')
    await page.click('button[type="submit"]')
    await page.waitForTimeout(2000) // Esperar propagació del canal

    // === USUARI 2: Rep el canal i obté la clau ===
    await page2.click('.channel-item:text("canal-secrt")')

    // Verificar que el receptor pot obtenir la clau del canal
    const channelKeyObtained = await page2.evaluate(async () => {
      // Aquest codi es simula com si el browser ho fes
      // En realitat el useChannelKey hook ho fa automàticament
      try {
        const keys = await fetch('/api/channels/canal-secrt/keys')
        const data = await keys.json()
        return data.length > 0
      } catch {
        return false
      }
    })
    expect(channelKeyObtained).toBe(true)

    // === USUARI 1: Envía missatge E2EE ===
    const secretMessage = 'Aquest és un missatge E2EE secret'
    await page.fill('.message-input', secretMessage)
    await page.press('.message-input', 'Enter')

    // Esperar broadcast
    await page.waitForTimeout(1000)

    // === USUARI 2: Rep i desencripta el missatge ===
    // El missatge ha d'aparèixer desxifrat a la interfície del receptor
    await expect(page2.locator('.message-bubble')).toContainText(secretMessage)

    // === VERIFICACIÓ CRÍTICA: Servidor NO pot llegir ===
    // Obtenir el missatge directament via API (simula servidor compromès)
    const serverPayload = await page.evaluate(async () => {
      // Simula que algú accedeix directament a la DB o API sense tenir la clau
      const res = await fetch('/api/channels/canal-secrt/messages')
      const data = await res.json()
      return data[0]?.encryptedPayload
    })

    // El payload ha d'estar xifrat
    expect(serverPayload).not.toBe(secretMessage)
    expect(serverPayload.length).toBeGreaterThan(50)

    // === VERIFICACIÓ: El ciphertext és unique per missatge ===
    await page.fill('.message-input', 'Aquest és un altre missatge secret')
    await page.press('.message-input', 'Enter')
    await page.waitForTimeout(1000)

    const serverPayload2 = await page.evaluate(async () => {
      const res = await fetch('/api/channels/canal-secrt/messages')
      const data = await res.json()
      return data[0]?.encryptedPayload
    })

    // Dos missatges diferents han de tenir payloads diferents (IV aleatori)
    expect(serverPayload).not.toBe(serverPayload2)

    await contextB.close()
  })

  test('usuari sense accés no pot veure missatges', async ({ page }) => {
    // === USUARI A: Crea canal asimètric ===
    await registerAndLogin(page, 'user_a', 'Password123!')
    await createChannel(page, 'privat-a', 'asymmetric')

    // === USUARI B: No té accés ===
    const contextB = await page.browserContext().newContext()
    const pageB = await contextB.newPage()
    await registerAndLogin(pageB, 'user_b', 'Password123!')

    // Usuari B NO ha de veure el canal privat
    await expect(pageB.locator('.channel-item:text("privat-a")')).not.toBeVisible()

    // Intentar accedir via URL directe ha de fallar
    await pageB.goto('/app/server/test/channel/privat-a')
    await expect(pageB.locator('.message-bubble')).not.toBeVisible()

    await contextB.close()
  })

  test('multi-dispositiu: missatge arriba a tots els dispositius del receptor', async ({ page }) => {
    // === USUARI 1: Crea canal + envía ===
    await registerAndLogin(page, 'multi_sender', 'Password123!')
    await createChannel(page, 'multi-channel', 'asymmetric')

    const testMessage = 'Missatge multi-dispositiu'
    await page.fill('.message-input', testMessage)
    await page.press('.message-input', 'Enter')

    // === USUARI 2: Dos dispositius (dues pàgines diferents) ===
    const contextB = await page.browserContext().newContext()
    const pageB1 = await contextB.newPage()
    const pageB2 = await contextB.newPage()

    // Login a ambdós dispositius
    await registerAndLogin(pageB1, 'multi_receiver', 'Password123!')
    await registerAndLogin(pageB2, 'multi_receiver', 'Password123!')

    // Convidar tots dos dispositius (via API, ja que cada dispositiu té el seu deviceId)
    // ...

    // Ambdós dispositius han de rebre el missatge
    await expect(pageB1.locator('.message-bubble')).toContainText(testMessage)
    await expect(pageB2.locator('.message-bubble')).toContainText(testMessage)

    await contextB.close()
  })

  test('forward secrecy: un dispositiu revocat no pot llegir nous missatges', async ({ page }) => {
    // ...
  })
})

// Helpers
async function registerAndLogin(page, username, password) {
  await page.goto('/login')
  try {
    await page.click('text=Registra\'t')
    await page.fill('input[name="username"]', username)
    await page.fill('input[name="password"]', password)
    await page.click('button[type="submit"]')
    await page.waitForURL('/app/**')
  } catch {
    // Ja existeix, fer login
    await page.goto('/login')
    await page.fill('input[name="username"]', username)
    await page.fill('input[name="password"]', password)
    await page.click('button[type="submit"]')
    await page.waitForURL('/app/**')
  }
}

async function expectKyberKeypairExists(page) {
  const exists = await page.evaluate(async () => {
    return new Promise((resolve) => {
      const req = indexedDB.open('chillgroup-store', 1)
      req.onsuccess = () => {
        const tx = req.result.transaction('keypairs', 'readonly')
        const count = tx.objectStore('keypairs').count()
        count.onsuccess = () => resolve(count.result > 0)
      }
    })
  })
  expect(exists).toBe(true)
}

async function createChannel(page, name, encryption) {
  await page.click('.new-channel-btn')
  await page.fill('input[name="channel-name"]', name)
  await page.selectOption('select[name="encryption"]', encryption)
  await page.click('button[type="submit"]')
  await page.waitForTimeout(500)
}
```

### 6. voice-e2ee.spec.ts — E2EE de Veu (LiveKit)

```typescript
// tests/e2e/voice-e2ee.spec.ts
import { test, expect } from '@playwright/test'

test.describe('E2EE de Veu (LiveKit)', () => {
  test('participants en canal de veu E2EE no poden escoltar sense session key', async ({ page }) => {
    // === USUARI 1: Crea canal de veu + E2EE ===
    await registerAndLogin(page, 'voice_sender', 'Password123!')
    await createChannel(page, 'veure-cript', 'voice', 'asymmetric')

    // Unir-se al canal de veu
    await page.click('.channel-item.voice-channel')
    await page.waitForSelector('.voice-area')
    await page.click('.voice-btn-mic')

    // Verificar que el canal té E2EE de veu
    await expect(page.locator('.voice-channel-header')).toContainText('🔒')

    // === USUARI 2: Es uneix sense session key ===
    const contextB = await page.browserContext().newContext()
    const pageB = await contextB.newPage()
    await registerAndLogin(pageB, 'voice_receiver', 'Password123!')
    await pageB.click('.channel-item.voice-channel')
    await pageB.waitForSelector('.voice-area')

    // Verificar que l'E2EE està activat però sense key
    const e2eeStatus = await pageB.evaluate(() => {
      // LiveKit exposeix l'estat E2EE
      return document.querySelector('.e2ee-status')?.textContent
    })
    expect(e2eeStatus).toContain('E2EE')

    await contextB.close()
  })

  test('session key distribuïda via canal de text funciona', async ({ page }) => {
    // ...
  })
})
```

## Tests Unitaris Rust (Server)

### crypto/kyber_test.rs

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use x25519_dilithium::{KeyPair, EncapsulatingKey, DecapsulatingKey};

    #[test]
    fn test_keygen_generates_correct_size_keys() {
        let keypair = KeyPair::generate(&mut rand::rngs::OsRng);
        let encapsulating: EncapsulatingKey = keypair.encapsulating_key();
        let public_key_bytes: Vec<u8> = (&encapsulating).into();

        // Kyber-1024 public key = 1568 bytes
        assert_eq!(public_key_bytes.len(), 1568);
    }

    #[test]
    fn test_encapsulate_decapsulate_roundtrip() {
        let keypair = KeyPair::generate(&mut rand::rngs::OsRng);
        let encapsulating: EncapsulatingKey = keypair.encapsulating_key();
        let public_key_bytes: Vec<u8> = (&encapsulating).into();

        // Channel key (32 bytes = AES-256)
        let channel_key: [u8; 32] = rand::thread_rng().gen();

        // Encapsular
        let (shared_secret, ciphertext) = encapsulating
            .encapsulate(&mut rand::rngs::OsRng)
            .unwrap();

        // Verificar tamaños
        assert_eq!(ciphertext.len(), 1088); // Kyber-1024 ciphertext
        assert_eq!(shared_secret.len(), 32); // Shared secret 256 bits

        // Desencapsular
        let decapsulating: DecapsulatingKey = keypair.decapsulating_key();
        let decoded_secret = decapsulating.decapsulate(&ciphertext).unwrap();

        // Els secrets han de coincidir
        assert_eq!(shared_secret, decoded_secret);
    }

    #[test]
    fn test_different_keypairs_cannot_decapsulate() {
        let keypair1 = KeyPair::generate(&mut rand::rngs::OsRng);
        let keypair2 = KeyPair::generate(&mut rand::rngs::OsRng);

        let encapsulating1: EncapsulatingKey = keypair1.encapsulating_key();
        let (_shared_secret, ciphertext) = encapsulating1
            .encapsulate(&mut rand::rngs::OsRng)
            .unwrap();

        // Desencapsular amb un keypair diferent HAU de fallar
        let decapsulating2: DecapsulatingKey = keypair2.decapsulating_key();
        let result = decapsulating2.decapsulate(&ciphertext);

        // Ha de retornar error (ciphertext no correspon a keypair2)
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_kek_deterministic() {
        let shared_secret = vec![0u8; 32];
        let channel_id = Uuid::new_v4();

        let kek1 = derive_kek(&shared_secret, channel_id);
        let kek2 = derive_kek(&shared_secret, channel_id);

        // HKF ha de ser determinístic
        assert_eq!(kek1, kek2);
    }
}
```

### integration/crypto_flow_test.rs

```rust
#[cfg(test)]
mod tests {
    use crate::db::test_db;
    use crate::services::channel_service::ChannelService;
    use crate::services::crypto_service::CryptoService;
    use uuid::Uuid;

    #[tokio::test]
    async fn full_e2ee_flow_create_invite_receive_message() {
        let db = test_db().await;
        let crypto = CryptoService::new();

        // 1. Crear 2 usuaris amb dispositius
        let user1_id = Uuid::new_v4();
        let user2_id = Uuid::new_v4();
        let device1_id = Uuid::new_v4();
        let device2_id = Uuid::new_v4();

        // ... insert users and devices with Kyber keypairs

        // 2. Creador crea canal asimètric
        let channel_id = Uuid::new_v4();
        let channel_key = crypto.generate_channel_key().await.unwrap();

        // 3. Convidar usuari 2 (server emmagatzema clau encriptada)
        let encrypted_key = crypto
            .encrypt_channel_key_for_device(
                &channel_key,
                &device2_public_key,
                channel_id,
            )
            .await
            .unwrap();

        db.channel_keys().insert(
            channel_id,
            device2_id,
            &encrypted_key.encrypted_key,
            &encrypted_key.kem_ciphertext,
        ).await.unwrap();

        // 4. Usuari 2 obté i desencripta la clau
        let retrieved_key = db.channel_keys().get_for_device(channel_id, device2_id).await.unwrap();
        let decrypted_key = crypto
            .decrypt_channel_key_for_device(
                &retrieved_key,
                &device2_secret_key,
                channel_id,
            )
            .await
            .unwrap();

        // La clau desencriptada ha de ser la mateixa
        assert_eq!(channel_key.as_ref(), decrypted_key.as_ref());

        // 5. Usuari 1 envía missatge xifrat
        let plaintext = "Missatge secret E2EE";
        let encrypted_msg = crypto.encrypt_message(&channel_key, plaintext).await.unwrap();

        // 6. Usuari 2 desencripta el missatge
        let decrypted_msg = crypto.decrypt_message(&decrypted_key, &encrypted_msg).await.unwrap();
        assert_eq!(decrypted_msg, plaintext);

        // 7. Verificar que servidor no pot llegir
        assert_ne!(encrypted_msg.encrypted_payload, plaintext);
    }
}
```

## Tests Frontend (Vitest + React Testing Library)

### crypto.test.ts

```typescript
import { describe, it, expect, beforeEach } from 'vitest'
import {
  generateKyberKeyPair,
  kemEncapsulate,
  kemDecapsulate,
  encryptMessage,
  decryptMessage,
} from '../lib/crypto'

describe('Crypto Module', () => {
  let keypair: Awaited<ReturnType<typeof generateKyberKeyPair>>

  beforeEach(async () => {
    keypair = await generateKyberKeyPair()
  })

  describe('generateKyberKeyPair', () => {
    it('genera claus de mida correcta', () => {
      // publicKey = 1568 bytes
      const decoded = atob(keypair.publicKey)
      expect(decoded.length).toBe(1568)
    })
  })

  describe('KEM Encapsulate/Decapsulate', () => {
    it('encapsula i desencapsula una clau channelKey', async () => {
      const channelKey = new Uint8Array(32)
      crypto.getRandomValues(channelKey)

      const { encryptedKey, ciphertext } = await kemEncapsulate(
        keypair.publicKey,
        channelKey
      )

      // Desencriptar amb el secret key
      const decrypted = await kemDecapsulate(
        keypair.secretKey,
        ciphertext,
        encryptedKey
      )

      expect(decrypted).toEqual(channelKey)
    })

    it('falla amb un secret key incorrecte', async () => {
      const channelKey = new Uint8Array(32)
      crypto.getRandomValues(channelKey)

      // Generar un keypair diferent per al "wrong key"
      const wrongKeypair = await generateKyberKeyPair()

      const { encryptedKey, ciphertext } = await kemEncapsulate(
        keypair.publicKey,
        channelKey
      )

      // Desencriptar amb keypair incorrecte HA de fallar
      await expect(
        kemDecapsulate(wrongKeypair.secretKey, ciphertext, encryptedKey)
      ).rejects.toThrow()
    })
  })

  describe('AES-GCM Encrypt/Decrypt', () => {
    it('xifra i desxifra un missatge', async () => {
      const key = await crypto.subtle.generateKey(
        { name: 'AES-GCM', length: 256 },
        true,
        ['encrypt', 'decrypt']
      )

      const plaintext = 'Missatge de prova E2EE'
      const encrypted = await encryptMessage(key, plaintext)

      expect(encrypted.encrypted).not.toBe(plaintext)
      expect(encrypted.iv.length).toBeGreaterThan(0)

      const decrypted = await decryptMessage(key, encrypted.encrypted, encrypted.iv)
      expect(decrypted).toBe(plaintext)
    })

    it('dos encriptacions del mateix text tenen IV diferent', async () => {
      const key = await crypto.subtle.generateKey(
        { name: 'AES-GCM', length: 256 },
        true,
        ['encrypt', 'decrypt']
      )

      const msg = 'Missatge idèntic'
      const enc1 = await encryptMessage(key, msg)
      const enc2 = await encryptMessage(key, msg)

      // Més encriptats però IV diferent
      expect(enc1.encrypted).not.toBe(enc2.encrypted)
      expect(enc1.iv).not.toBe(enc2.iv)
    })
  })
})
```

### storage.test.ts

```typescript
import { describe, it, expect, beforeEach } from 'vitest'
import {
  storeKeypair,
  getKeypair,
  storeChannelKey,
  getChannelKey,
  deleteChannelKey,
} from '../lib/storage'

describe('IndexedDB Storage', () => {
  beforeEach(async () => {
    // Netejar BD abans de cada test
    const req = indexedDB.open('chillgroup-store', 1)
    req.onupgradeneeded = () => {
      const db = req.result
      if (db.objectStoreNames.contains('keypairs')) {
        db.deleteObjectStore('keypairs')
      }
      if (db.objectStoreNames.contains('channelKeysBytes')) {
        db.deleteObjectStore('channelKeysBytes')
      }
    }
    // ...
  })

  it('guarda i recupera un keypair', async () => {
    const secretKey = new Uint8Array(3168)
    crypto.getRandomValues(secretKey)

    await storeKeypair('device-1', secretKey)

    const retrieved = await getKeypair('device-1')
    expect(retrieved).toEqual(secretKey)
  })

  it('retorna null si el keypair no existeix', async () => {
    const result = await getKeypair('nonexistent-device')
    expect(result).toBeNull()
  })

  it('guarda i recupera un canalKey', async () => {
    const channelKey = new Uint8Array(32)
    crypto.getRandomValues(channelKey)

    await storeChannelKey('channel-1', channelKey, 'asymmetric')

    const retrieved = await getChannelKey('channel-1')
    expect(retrieved).toEqual(channelKey)
  })

  it('elimina un canalKey', async () => {
    const key = new Uint8Array(32)
    await storeChannelKey('channel-1', key, 'symmetric')
    await deleteChannelKey('channel-1')

    const retrieved = await getChannelKey('channel-1')
    expect(retrieved).toBeNull()
  })
})
```

## CI/CD Integration (GitHub Actions)

```yaml
# .github/workflows/test.yml
name: Tests

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  rust-unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      - name: Run Rust unit tests
        run: cd server && cargo test --no-fail-fast

  rust-integration-tests:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_DB: chillgroup_test
          POSTGRES_USER: chillgroup
          POSTGRES_PASSWORD: chillgroup
        ports:
          - 5432:5432
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust + PostgreSQL client
        run: |
          curl -L https://install.python-poetry.org | python3 -
          apt-get update && apt-get install -y libpq-dev
      - name: Run Rust integration tests
        run: |
          cd server
          export DATABASE_URL=postgresql://chillgroup:chillgroup@localhost:5432/chillgroup_test
          cargo test --test integration

  frontend-unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
      - name: Install dependencies
        run: cd frontend && pnpm install
      - name: Run frontend unit tests
        run: cd frontend && pnpm test -- --run

  playwright-e2e:
    runs-on: ubuntu-latest
    needs: [rust-unit-tests, frontend-unit-tests]
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      - name: Install Playwright browsers
        run: npx playwright install --with-deps chromium firefox
      - name: Install dependencies
        run: pnpm install
      - name: Build server
        run: cd server && cargo build --release
      - name: Run Playwright E2E tests
        run: npm run test:ci
      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: playwright-report
          path: playwright-report/
          retention-days: 30
```

## Resum de Cobertura Objectiu

| Component | Tipus de test | Eina | Cobertura objectiu |
|-----------|---------------|------|-------------------|
| Crypto module | Unitari | Rust `cargo test` | 100% |
| Service layer | Integració | `axum-test` + SQLx | 80%+ |
| Repository layer | Integració | SQLx + DB real | 100% |
| API routes | Integració | `axum-test` | 80%+ |
| Frontend hooks | Unitari | Vitest + RTL | 80%+ |
| Frontend components | Unitari | Vitest + RTL | 70%+ |
| E2E login/auth | E2E | Playwright | 100% |
| E2E missatges | E2E | Playwright | 100% |
| E2E E2EE none | E2E | Playwright | 100% |
| E2E E2EE symmetric | E2E | Playwright | 100% |
| E2E E2EE asymmetric | E2E | Playwright | 100% |
| E2E voice E2EE | E2E | Playwright | 100% |
| E2E multi-dispositiu | E2E | Playwright | 100% |

## Execució Ràpida

```bash
# Test unitaris Rust
make test-server

# Test unitaris frontend
make test-frontend

# Tests E2E complets (frontend + backend)
make test-e2e

# Només tests d'encriptació
make test-encryption

# Només test asimètric (E2EE)
make test-asymmetric

# En mode headed (veure el browser)
make dev-test FILE=e2e/encryption/asymmetric.spec.ts

# Veure report
npm run test:report
```
