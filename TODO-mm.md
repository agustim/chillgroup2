# TODO — ChillGroup v2

> **Convenció:** cada ítem inclou el seu criteri de fet i els tests que cal afegir.
> Cap feature es considera acabada sense tests.

---

## En curs (branca `dev`)

Refactor del subsistema de criptografia i permisos al frontend:
- `frontend/src/lib/crypto.ts` — revisió de la capa de criptografia
- `frontend/src/lib/channel-crypto.ts` — revisió de lògica de claus de canal
- `frontend/src/lib/device-identity.ts` / `device-keys.ts` — revisió d'identitat de dispositiu
- `frontend/src/lib/storage.ts` / `logger.ts` / `api.ts` — canvis de suport
- `frontend/src/components/modals/PermissionsModal.tsx` — UI de permisos de canal
- `frontend/src/components/sidebar/ServerBar.tsx` — canvis de sidebar

**Tests pendents per a "en curs":**
- [ ] Vitest: `crypto.test.ts` — cobreix els nous paths canviats
- [ ] Vitest: `channel-crypto.test.ts` — encrypt/decrypt round-trip per als 3 nivells
- [ ] Vitest: `device-identity.test.ts` / `device-keys.test.ts` — cobreix setup i rotació de dispositiu
- [ ] Vitest: `storage.test.ts` — cobreix els nous stores si han canviat

---

## P0 — Funcionalitats bàsiques que falten

### 1. Sortir d'un servidor (Leave Server)

**Backend:**
- [ ] Endpoint `DELETE /api/servers/:serverId/members/me`
- [ ] Si l'usuari és l'únic admin: retorna `409` amb missatge d'advertència; el client ha de confirmar transfer·lència o eliminació del servidor abans de poder sortir
- [ ] Si l'usuari és l'owner i hi ha altres membres: transferir ownership o bloquejar

**Frontend:**
- [ ] Opció "Sortir del servidor" al menú contextual del servidor (ServerBar)
- [ ] Modal de confirmació; avisar si l'usuari és l'últim admin

**Tests (backend):**
- [ ] `delete_server_member_self_succeeds` — membre normal pot sortir
- [ ] `delete_server_member_self_last_admin_blocked` — últim admin no pot sortir sense transferir
- [ ] `delete_server_member_self_owner_blocked_if_not_last` — owner amb membres no pot sortir sense transferir

**Tests (frontend - Vitest/RTL):**
- [ ] `ServerBar` — mostra opció "Sortir" per a servidors on l'usuari no és owner
- [ ] Modal de confirmació es mostra amb advertència si és últim admin

---

### 2. Invitació a servidor amb acceptació (no auto-join)

Ara `POST /api/servers/:serverId/members` afegeix l'usuari directament. Cal un flux de convit amb acceptació.

**Backend:**
- [ ] Taula `server_invitations` (migració): `id`, `server_id`, `inviter_id`, `invitee_id`, `status` (pending/accepted/declined), `created_at`, `expires_at`
- [ ] `POST /api/servers/:serverId/invitations` — crear invitació pendent (envia event Socket.IO a `user:<invitee_id>`)
- [ ] `POST /api/servers/:serverId/invitations/:invitationId/accept` — acceptar i afegir membre
- [ ] `POST /api/servers/:serverId/invitations/:invitationId/decline` — declinar
- [ ] `GET /api/user/me/server-invitations` — llistar invitacions pendents de l'usuari
- [ ] L'endpoint existent `POST /api/servers/:serverId/members` es manté per compatibilitat però pot passar a deprecated o redirigir al nou flux

**Frontend:**
- [ ] Notificació/badge a la UI quan arriba una invitació (via Socket.IO)
- [ ] Modal o secció "Invitacions pendents" per acceptar/declinar
- [ ] El ServerBar es refresca en acceptar

**Tests (backend):**
- [ ] `invite_to_server_creates_pending_invitation`
- [ ] `accept_server_invitation_adds_member`
- [ ] `decline_server_invitation_removes_pending`
- [ ] `accept_expired_invitation_rejected`
- [ ] `list_pending_invitations_for_user`

**Tests (frontend - Playwright E2E):**
- [ ] `server-invitation.spec.ts`: user1 convida user2 → user2 veu notificació → accepta → ambdós dins el servidor

---

### 3. Docker mínim per a desenvolupament local

**Infraestructura:**
- [ ] Crear `docker-compose.minimal.yml` amb només `livekit`, `rustfs`, `rustfs-init`, `rustfs-cors-init`
- [ ] Reutilitzar `.env.compose` i `.env.compose.local` (no duplicar configuració)

**Criteri de fet:** `docker compose -f docker-compose.minimal.yml up` arrenca sense Postgres ni app i es pot fer upload a bucket + connexió LiveKit.

**Tests:**
- [ ] Test de smoke manual (documentat al `definitions/DEVELOPMENT.md`): curl a endpoint S3 retorna 200

---

### 4. Límit global de mida per fitxer (`MAX_FILE_SIZE`)

**Backend:**
- [ ] Afegir `max_file_size_bytes` a `Config` (`server/src/config.rs`) amb default segur (p. ex. 100 MB)
- [ ] Validar `req.size_bytes <= max_file_size_bytes` a `init_attachment` (`server/src/routes/attachments.rs`)
- [ ] Retornar `AppError` clar (`413 Payload Too Large` + codi `FileTooLarge`)

**Tests (backend):**
- [ ] `init_attachment_exceeds_max_size_rejected` — upload > `MAX_FILE_SIZE` retorna 413
- [ ] `init_attachment_within_max_size_succeeds` — upload <= `MAX_FILE_SIZE` funciona
- [ ] `init_attachment_default_size_sensible` — sense `MAX_FILE_SIZE` al .env, el default aplica

---

### 5. Quotes d'S3 per pla (espai + transferència)

**Backend — model:**
- [ ] Migració SQL: afegir a `plans` les columnes `max_storage_bytes BIGINT NOT NULL DEFAULT -1` i `max_transfer_bytes_monthly BIGINT NOT NULL DEFAULT -1`
- [ ] Actualitzar valors dels plans per defecte: Free (10 GB espai, 100 GB transfer), Pro (50 GB, 500 GB), Enterprise (-1, -1)
- [ ] Exposar camps nous a `GET /api/plans` i `GET /api/user/me/plan`

**Backend — persistència de consum:**
- [ ] Migració SQL: nova taula `user_storage_usage_monthly` (`user_id`, `year_month CHAR(7)`, `stored_bytes BIGINT`, `transfer_bytes BIGINT`, timestamps + UNIQUE `(user_id, year_month)`)
- [ ] Incrementar `stored_bytes` en `complete_attachment`
- [ ] Incrementar `transfer_bytes` en descàrregues (`download_attachment` / `download_attachment_proxy`)
- [ ] Política de decrement de `stored_bytes` en esborrar adjunts (si l'esborrat d'adjunts no existeix, marcar com a dependència)

**Backend — enforcement:**
- [ ] A `init_attachment`: comprovar `stored_bytes + req.size_bytes <= max_storage_bytes` (skip si -1)
- [ ] A endpoints de descàrrega: comprovar `transfer_bytes + size_bytes <= max_transfer_bytes_monthly` (skip si -1)
- [ ] Retornar `429` amb codi `StorageQuotaExceeded` o `TransferQuotaExceeded`

**Tests (backend):**
- [ ] `init_attachment_storage_quota_exceeded` — upload bloquejat quan espai exhaurit
- [ ] `download_transfer_quota_exceeded` — descàrrega bloquejada quan transferència exhaurida
- [ ] `unlimited_plan_bypasses_quotas` — `-1` no bloqueja
- [ ] `monthly_usage_increments_on_upload_complete`
- [ ] `monthly_usage_increments_on_download`
- [ ] `usage_visible_in_user_plan_endpoint`

---

## P1 — Funcionalitats importants

### 6. Missatges Directes (DM) v2

El model v2 (canal 1:1 asimètric `scope=dm`) ja té les rutes definides a `messages.rs`, però:

**Backend:**
- [ ] `list_conversations` (`GET /api/conversations`) té un TODO: implementar amb `scope=dm`
- [ ] Migració: afegir `scope`, `dm_user_a_id`, `dm_user_b_id` a `channels` + nullable `server_id` + índex únic de parella (veure `definitions/DM.md`)
- [ ] `POST /api/dm/channels/open` — idempotent: si ja existeix, retorna `created: false`
- [ ] Bootstrap `channel_key_versions` en crear DM
- [ ] `GET /api/dm/channels` — llistar converses amb peer info + `unreadCount`
- [ ] `PUT /api/dm/channels/:id/settings` — canviar `message_ttl`
- [ ] `POST /api/dm/channels/:id/keys/rotate`
- [ ] Mantenir compatibilitat dels endpoints legacy (`/api/direct-messages`, `/api/conversations`)

**Frontend:**
- [ ] Llista de converses DM (sidebar o secció dedicada)
- [ ] Vista de conversa DM (reutilitzant `MainContent` + crypto asimètric)
- [ ] Modal de configuració de TTL de DM
- [ ] Distribució de bundles asimètrics en obrir DM

**Tests (backend):**
- [ ] `open_dm_channel_creates_new`
- [ ] `open_dm_channel_idempotent_returns_existing`
- [ ] `dm_channel_third_user_cannot_access`
- [ ] `dm_message_ttl_applied_to_messages`
- [ ] `dm_key_rotation_creates_new_version`
- [ ] `list_dm_channels_returns_peer_info`

**Tests (Playwright E2E):**
- [ ] `dm.spec.ts`: user1 obre DM amb user2 → intercanvien missatges E2EE → tercer usuari no pot accedir

---

### 7. Quotes de streaming LiveKit per pla

**Backend:**
- [ ] Migració SQL: afegir `max_streaming_hours_monthly INT NOT NULL DEFAULT -1` a `plans`
- [ ] Valors: Free (10h), Pro (50h), Enterprise (-1)
- [ ] Taula `user_streaming_usage_monthly` (`user_id`, `year_month`, `streaming_seconds BIGINT`)
- [ ] Incrementar consum en `POST /api/livekit/token` o via webhook de LiveKit
- [ ] Bloquejar token si quota esgotada (retornar `429 StreamingQuotaExceeded`)

**Tests (backend):**
- [ ] `livekit_token_blocked_when_streaming_quota_exceeded`
- [ ] `streaming_usage_accumulated_per_month`
- [ ] `unlimited_streaming_bypasses_quota`

---

### 8. Notificacions de límit de pla (80% / 90% / 100%)

**Backend:**
- [ ] Afegir camps `warning_sent_at_80`, `warning_sent_at_90` a `user_storage_usage_monthly` (o taula separada d'avisos)
- [ ] En cada increment de consum: si es supera el 80% o 90%, emetre event Socket.IO `quota_warning` a `user:<id>`

**Frontend:**
- [ ] Handler de `quota_warning` a Socket.IO
- [ ] Banner o toast informatiu quan es rep avís de quota
- [ ] Bloqueig UI clar quan es supera el 100% (nova creació retorna error del backend)

**Tests (backend):**
- [ ] `quota_warning_emitted_at_80_percent`
- [ ] `quota_warning_emitted_at_90_percent`
- [ ] Avís no es repeteix si ja s'ha enviat

**Tests (frontend - Vitest/RTL):**
- [ ] Banner es mostra en rebre event `quota_warning`

---

### 9. Pàgina d'autoservei de subscripció (usuari)

**Frontend:**
- [ ] Nova ruta `/app/settings/plan`
- [ ] Mostra el pla actual amb límits i ús actual (cridant `GET /api/user/me/plan`)
- [ ] Mostra els plans disponibles (`GET /api/plans`)
- [ ] Botó "Canviar pla" (pendent integració de pagament — por ara, demana a admin)

**Tests (frontend - Vitest/RTL):**
- [ ] Renderitza pla actual amb límits correctes
- [ ] Mostra ús actual de recursos

---

## P2 — Qualitat i documentació

### 10. Tests E2E de Playwright que falten

Segons `definitions/TESTING.md`, molts tests E2E estan definits però no existeixen:

- [ ] `frontend/tests/e2e/servers.spec.mjs` — CRUD de servidors
- [ ] `frontend/tests/e2e/channels.spec.mjs` — crear, configurar, eliminar canals; TTL
- [ ] `frontend/tests/e2e/messages.spec.mjs` — enviar, editar, eliminar; TTL
- [ ] `frontend/tests/e2e/encryption/none.spec.mjs` — canal sense encriptació
- [ ] `frontend/tests/e2e/encryption/symmetric.spec.mjs` — clau simètrica
- [ ] `frontend/tests/e2e/encryption/asymmetric.spec.mjs` — E2EE complet (crític)
- [ ] `frontend/tests/e2e/friends.spec.mjs` — afegir, llistar, eliminar amics
- [ ] `frontend/tests/e2e/voice.spec.mjs` — connexió a canal de veu (LiveKit)
- [ ] `frontend/tests/e2e/permissions.spec.mjs` — permisos explícits de canal

---

### 11. Tests unitaris Rust que falten

- [ ] `server/tests/unit/crypto/kyber_test.rs` — keygen, encapsulate/decapsulate round-trip, cross-keypair failure
- [ ] `server/tests/unit/crypto/aes_gcm_test.rs` — encrypt/decrypt, IV únic
- [ ] `server/tests/unit/crypto/channel_keys_test.rs` — rotació i distribució
- [ ] `server/tests/unit/auth_test.rs` — hash, JWT
- [ ] `server/tests/integration/channel_flow_test.rs` — crear → convidar → accedir
- [ ] `server/tests/integration/crypto_flow_test.rs` — E2EE complet: crear canal → convidar → xifrar → desxifrar

---

### 12. Actualitzar documentació

- [ ] `definitions/API.md` — afegir endpoints nous (server invitations, leave server, quotes)
- [ ] `definitions/OVERVIEW.md` — actualitzar taula de límits amb quotes S3 i streaming
- [ ] `docs-site/` — sincronitzar amb `definitions/` (canals `ca` i `en`)

---

## Referència ràpida

| Prioritat | Feature | Backend | Frontend | Tests |
|-----------|---------|---------|----------|-------|
| P0 | Leave server | ❌ | ❌ | ❌ |
| P0 | Server invitation accept | ❌ | ❌ | ❌ |
| P0 | Docker minimal | ❌ | — | — |
| P0 | MAX_FILE_SIZE | ❌ | ❌ | ❌ |
| P0 | Quotes S3 (model + enforcement) | ❌ | ❌ | ❌ |
| P1 | DM v2 complet | ⚠️ parcial | ❌ | ❌ |
| P1 | Quotes LiveKit | ❌ | ❌ | ❌ |
| P1 | Notificacions de quota | ❌ | ❌ | ❌ |
| P1 | Pàgina subscripció usuari | — | ❌ | ❌ |
| P2 | Tests E2E Playwright | — | ❌ | — |
| P2 | Tests unitaris Rust crypto | ❌ | — | — |
| P2 | Actualitzar docs | — | — | — |

> ⚠️ = parcialment implementat (rutes existents però lògica incompleta)
