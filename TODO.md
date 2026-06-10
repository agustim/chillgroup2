# TODO — Auditoria de Seguretat i Millores (ChillGroup v2)

> Revisió: 2026-06-10. Abast: backend Rust (`server/`), frontend Vite (`frontend/`).
> Format: `[ ]` pendent · `[x]` fet. Referències `fitxer:línia`.

## Prioritat d'acció
1 → 2 → 3 → 4 (trenquen confidencialitat/auth directament), després 5-9.
Els punts 1, 2 i 3 són explotables avui per qualsevol compte vàlid.

---

## 🔴 Crítics / Alts

### [x] 1. `join-channel` per socket sense comprovació de permís
- **On:** `server/src/main.rs:466`
- **Problema:** el handler fa `socket.join("channel:{channelId}")` amb el `channelId` del client sense validar accés. `send_message` fa broadcast a `channel:{id}` (`messages.rs:359`). Qualsevol usuari autenticat pot unir-se a la room de qualsevol canal i rebre missatges en temps real. Canals `none` → text pla; canals xifrats → ciphertext + metadades (sender, timestamps).
- **Fix:** abans de `socket.join`, comprovar `get_channel_permission_level(channel_id, user_id) >= READ`, igual que els endpoints REST.

### [x] 2. IDOR als DM legacy
- **On:** `server/src/routes/messages.rs:569` (`list_direct_messages`), `messages.rs:509` (`send_direct_message`), router `messages.rs:1109-1111`
- **Problema:** `list_direct_messages` consulta tots els missatges amb `channel_id == nil` i només filtra per `channel_id == nil` — no filtra per remitent/destinatari. Qualsevol usuari autenticat llegeix els DM legacy de tothom. `send_direct_message` escriu sense noció de membres.
- **Fix:** eliminar els 3 endpoints legacy (`/api/direct-messages*`, `/api/conversations` ja substituïts per `/api/dm/channels`) o filtrar per `sender_user_id == claims.user_id OR recipient`.

### [x] 3. Claus privades E2EE en text pla a IndexedDB
- **On:** `frontend/src/lib/storage.ts:266`
- **Problema:** `upsertNamedKeypair` desa `kyberSecretKey`/`dsaSecretKey` com base64 sense xifrar. Existeix infra de vault (`local-vault.ts`, PBKDF2 600k + AES-GCM) però no s'usa per embolcallar les claus de dispositiu. Un XSS exfiltra totes les claus privades → trenca "zero-knowledge".
- **Fix:** xifrar `secretKey`/`dsaSecretKey` amb `encryptBytesForLocalVault()` abans de desar; desxifrar a `getNamedKeypair` amb el vault desbloquejat.

### [x] 4. `/api/auth/refresh` és un oracle de tokens
- **On:** `server/src/routes/auth.rs:557-566`
- **Problema:** endpoint públic (sense auth) que genera un JWT vàlid per a un `user_id` aleatori sense validar res. No reemet per la identitat existent.
- **Fix:** requerir el token actual (acceptant expirat), validar signatura, reemetre amb el mateix `user_id`/`device_id`. Verificar que usuari i dispositiu existeixen i no estan revocats.

---

## 🟠 Mitjans

### [x] 5. Sense revocació de tokens; `device.revoked` no s'aplica al middleware
- **On:** `server/src/middleware/auth.rs:86`
- **Problema:** `extract_claims` només verifica signatura + expiració. `jti` es genera (`auth.rs:80`) però mai es comprova. Dispositiu revocat o logout deixen el JWT vàlid 7 dies. Les queries de `devices.rs` filtren `revoked=0` però el middleware no.
- **Fix:** a cada request protegit, comprovar que `device_id` existeix i `revoked=false`. Opcional: blacklist de `jti` en logout (Redis).

### [x] 6. JWT HS256 amb secret compartit únic
- **On:** `server/src/middleware/auth.rs:55-60`, `main.rs:419`, `config.rs:162`
- **Problema:** docs prometen RS256; codi usa HS256. Mateix secret signa i verifica → si es filtra, qualsevol forja tokens admin. Sense separació access/refresh; access dura 7 dies.
- **Fix:** RS256 (privada signa, pública verifica) o access curt (~15 min) + refresh rotatori. Validar longitud mínima de `JWT_SECRET` a l'arrencada.

### [x] 7. CORS `permissive()`
- **On:** `server/src/main.rs:882` i `main.rs:914`
- **Problema:** `Access-Control-Allow-Origin: *` a tota l'API. Risc menor perquè auth és Bearer (no cookies), però permet a qualsevol web llegir respostes amb token robat.
- **Fix:** llista blanca d'orígens des de config (`ALLOWED_ORIGINS`).

### [x] 8. Rate limiting inexistent
- **On:** error types definits (`error.rs:51,121`) però mai cablejats
- **Problema:** cap middleware de límit. Login, register i missatges sense throttling tot i el que prometen els docs (`ARCHITECTURE.md:629`). Brute-force de passwords i spam de registre oberts.
- **Fix:** `tower_governor` o middleware propi per IP a `/auth/login` i `/auth/register`, i per usuari/canal a missatges.

### [x] 9. Token a `sessionStorage` + sense CSP
- **On:** `frontend/src/lib/api.ts:37`; servidor no emet headers de seguretat
- **Problema:** token accessible per JS → qualsevol XSS el roba. Sense `Content-Security-Policy`, `X-Frame-Options`, `X-Content-Type-Options`.
- **Fix:** capa `tower-http::set_header` amb CSP estricte i headers de seguretat. Valorar cookie `HttpOnly` (requereix protecció CSRF).

---

## 🟡 Menors / Millores

- [ ] **Codi d'invitació admin amb SHA-256 sense salt** — `crypto/hash.rs:33`. `ONE_ADMIN_INVITATION` escollit per humà → vulnerable a força bruta/rainbow. Usar Argon2 o exigir entropia alta.
- [ ] **LiveKit cobra 1h mínim per cada token** — `routes/livekit.rs:57`. Bug de quota: demanar token repetidament esgota streaming sense ús real.
- [ ] **Errors confusos** — username de longitud invàlida retorna `UsernameExists` (`auth.rs:114,318`). Dificulta debug.
- [ ] **Logs verbosos amb PII** — usernames i `user_id` a nivell `info!` arreu. Revisar abans de prod.
- [ ] **`.env` al directori** (no tracat per git, correcte) — confirmar `.gitignore` el cobreix de forma permanent.
- [ ] **Docs desfasats** — `ARCHITECTURE.md:152` diu `x25519-dilithium`; codi real usa crate `ml-kem 0.3.2`. Actualitzar també RS256 vs HS256 i rate limits "implementats".
