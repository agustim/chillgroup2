# TODO — ChillGroup v2

> **Convenció:** cada ítem inclou el seu criteri de fet i els tests que cal afegir.
> Cap feature es considera acabada sense tests.
> **Última revisió:** 2026-06-09

---

## En curs (branca `dev`)

Refactor del subsistema de criptografia i permisos al frontend:
- `frontend/src/lib/crypto.ts` — revisió de la capa de criptografia
- `frontend/src/lib/channel-crypto.ts` — revisió de lògica de claus de canal
- `frontend/src/lib/device-identity.ts` / `device-keys.ts` — revisió d'identitat de dispositiu
- `frontend/src/lib/storage.ts` / `logger.ts` / `api.ts` — canvis de suport
- `frontend/src/components/modals/PermissionsModal.tsx` — UI de permisos de canal
- `frontend/src/components/sidebar/ServerBar.tsx` — canvis de sidebar

**Tests per a "en curs":**
- [x] Vitest: `channel-crypto.test.ts` — encrypt/decrypt round-trip per als 3 nivells
- [x] Vitest: `device-identity.test.ts` / `device-keys.test.ts` — cobreix setup i rotació de dispositiu
- [x] Vitest: `storage.test.ts` — cobreix els nous stores
- [x] Vitest: `crypto.test.ts` — cobreix els nous paths canviats

---

## P0 — Funcionalitats bàsiques

### 1. Sortir d'un servidor (Leave Server) ✅ COMPLET

**Backend:**
- [x] Endpoint `DELETE /api/servers/:serverId/members/me`
- [x] Si l'usuari és l'únic admin: retorna `409` amb missatge d'advertència; el client ha de confirmar amb `?force=true`
- [x] Si l'usuari és l'owner: bloquejat (no pot sortir)

**Frontend:**
- [x] Opció "Sortir del servidor" al menú contextual del servidor (ServerBar)
- [x] Modal de confirmació; avisa si l'usuari és l'últim admin, amb opció de sortida forçosa

**Tests (backend):**
- [x] `leave_server_member_succeeds`
- [x] `leave_server_last_admin_blocked_without_force`
- [x] `leave_server_last_admin_allowed_with_force`
- [x] `leave_server_owner_is_blocked`
- [x] `leave_server_admin_with_other_admin_succeeds`
- [x] `leave_server_non_member_returns_not_found`

---

### 2. Invitació a servidor amb acceptació ✅ COMPLET

**Backend:**
- [x] Taula `server_invitations` (migració `20260124000000_create_server_invitations.sql`)
- [x] `POST /api/servers/:serverId/invitations` — crear invitació + event Socket.IO a `user:<invitee_id>`
- [x] `POST /api/servers/:serverId/invitations/:invitationId/accept`
- [x] `POST /api/servers/:serverId/invitations/:invitationId/decline`
- [x] `GET /api/user/me/server-invitations`

**Frontend:**
- [x] Handler Socket.IO `server-invitation` + badge de count a la UI
- [x] Modal `ServerInvitationsModal` per acceptar/declinar (amb serverName i inviterUsername)
- [x] ServerBar es refresca en acceptar

**Tests (backend):**
- [x] `create_invitation_succeeds_for_owner`
- [x] `create_invitation_fails_for_non_member`
- [x] `create_invitation_fails_if_already_member`
- [x] `accept_invitation_adds_member`
- [x] `decline_invitation_does_not_add_member`
- [x] `accept_invitation_wrong_user_is_forbidden`
- [x] `list_pending_invitations_for_user`

**Tests (frontend - Playwright E2E):**
- [ ] `server-invitation.spec.mjs`: user1 convida user2 → user2 veu notificació → accepta → ambdós dins el servidor

---

### 3. Docker mínim per a desenvolupament local ✅ COMPLET

**Infraestructura:**
- [x] `docker-compose.minimal.yml` creat (LiveKit + RustFS sense Postgres ni app)
- [x] Reutilitza `.env.compose` i `.env.compose.local`

**Tests:**
- [ ] Smoke test documentat a `definitions/DEVELOPMENT.md`: curl a endpoint S3 retorna 200

---

### 4. Límit global de mida per fitxer (`MAX_FILE_SIZE`) ✅ COMPLET

**Backend:**
- [x] `max_file_size_bytes` a `Config` (`server/src/config.rs`) amb default 100 MB
- [x] Validació `req.size_bytes <= max_file_size_bytes` a `init_attachment`
- [x] Error `AppError::FileTooLarge` → `413 Payload Too Large`

**Tests (backend):**
- [x] `init_attachment_file_too_large_returns_413`
- [x] `init_attachment_unlimited_max_zero`
- [x] `init_attachment_at_limit_succeeds`

---

### 5. Quotes d'S3 per pla (espai + transferència) ✅ COMPLET

**Backend — model:**
- [x] Migració `20260125000000_add_s3_quotas_to_plans.sql`: `max_storage_bytes`, `max_transfer_bytes_monthly` a `plans`
- [x] Valors: Free (10 GB / 100 GB), Pro (50 GB / 500 GB), Enterprise (-1 / -1)
- [x] `GET /api/plans` i `GET /api/user/me/plan-limits` exposen els camps nous

**Backend — persistència de consum:**
- [x] Migració `20260126000000_create_user_storage_usage_monthly.sql`: taula `user_storage_usage_monthly`
- [x] Increment `stored_bytes` a `complete_attachment`
- [x] Increment `transfer_bytes` a `download_attachment` / `download_attachment_proxy`
- [ ] Política de decrement de `stored_bytes` en esborrar adjunts (pendent si l'esborrat no existeix)

**Backend — enforcement:**
- [x] `init_attachment`: comprova `stored_bytes + req.size_bytes <= max_storage_bytes` (skip si -1)
- [x] Descàrregues: comprova `transfer_bytes + size <= max_transfer_bytes_monthly` (skip si -1)
- [x] Errors `StorageQuotaExceeded` (429) i `TransferQuotaExceeded` (429)

**Tests (backend):**
- [x] `init_attachment_storage_quota_exceeded_returns_error`
- [x] `download_transfer_quota_exceeded_returns_error`
- [x] `unlimited_plan_bypasses_quotas` (-1 no bloqueja)
- [x] `monthly_usage_increments_on_upload_complete`
- [x] `monthly_usage_increments_on_download`
- [x] `usage_visible_in_user_plan_endpoint`

---

## P1 — Funcionalitats importants

### 6. Missatges Directes (DM) v2 ✅ COMPLET

**Backend:**
- [x] Migració `20260112000000_prepare_dm_channels.sql`: `scope`, `dm_user_a_id`, `dm_user_b_id` a `channels` + índex únic
- [x] `POST /api/dm/channels/open` — idempotent, retorna `created: false` si ja existeix
- [x] Bootstrap `channel_key_versions` en crear DM
- [x] `GET /api/dm/channels` — llistar converses amb peer info + `unreadCount`
- [x] `PUT /api/dm/channels/:id/settings` — canviar `message_ttl`
- [x] `POST /api/dm/channels/:id/keys/rotate`
- [x] Compatibilitat endpoints legacy

**Frontend:**
- [x] Llista de converses DM (sidebar o secció dedicada)
- [x] Vista de conversa DM (reutilitzant MainContent + crypto asimètric)
- [ ] Modal de configuració de TTL de DM ⚠️ `handleUpdateTTL` existeix però sense modal dedicat
- [x] Distribució de bundles asimètrics en obrir DM

**Tests (backend):**
- [x] `open_dm_channel_creates_new`
- [x] `open_dm_channel_idempotent_returns_existing`
- [x] `dm_channel_third_user_cannot_access`
- [x] `dm_message_ttl_applied_to_messages`
- [x] `dm_key_rotation_creates_new_version`
- [x] `list_dm_channels_returns_peer_info`

**Tests (Playwright E2E):**
- [ ] `dm.spec.mjs`: user1 obre DM amb user2 → intercanvien missatges E2EE → tercer usuari no pot accedir

---

### 7. Quotes de streaming LiveKit per pla ✅ COMPLET (backend)

**Backend:**
- [x] Migració `20260127000000_add_streaming_quota_to_plans.sql`: `max_streaming_hours_monthly` a `plans`
- [x] Valors: Free (10h), Pro (50h), Enterprise (-1)
- [x] Migració `20260128000000_create_user_streaming_usage_monthly.sql`: taula `user_streaming_usage_monthly`
- [x] Enforcement al token LiveKit: bloqueja si quota esgotada (`429 StreamingQuotaExceeded`)
- [x] Carrega 1h de crèdit per token generat

**Tests (backend):**
- [x] `livekit_token_blocked_when_streaming_quota_exhausted`
- [x] `streaming_usage_accumulated_per_month`
- [x] `unlimited_streaming_bypasses_quota`

---

### 8. Notificacions de límit de pla (80% / 90% / 100%) ✅ COMPLET (backend)

**Backend:**
- [x] Camps `warning_sent_at_80`, `warning_sent_at_90` a `user_storage_usage_monthly`
- [x] En `complete_attachment`: emet event Socket.IO `quota_warning` a `user:<id>` al superar 80% o 90%
- [x] Avís no es repeteix si ja s'ha enviat

**Frontend:**
- [x] Handler de `quota_warning` a Socket.IO (AppLayout.tsx)
- [x] Variable d'estat `quotaWarning` actualitzada
- [x] Banner o toast visible a la UI quan es rep avís de quota (`AppLayout.tsx:284-289`)
- [ ] Bloqueig UI clar quan es supera el 100%

**Tests (backend):**
- [x] `quota_warning_emitted_at_80_percent`
- [x] `quota_warning_not_repeated_once_set`

---

### 9. Pàgina d'autoservei de subscripció (usuari) ❌ NO IMPLEMENTAT

**Frontend:**
- [ ] Nova ruta `/app/settings/plan`
- [ ] Mostra el pla actual amb límits i ús actual (`GET /api/user/me/plan-limits`)
- [ ] Mostra els plans disponibles (`GET /api/plans`)
- [ ] Botó "Canviar pla" (pendent integració de pagament — de moment, demana a admin)

**Tests (frontend - Vitest/RTL):**
- [ ] Renderitza pla actual amb límits correctes
- [ ] Mostra ús actual de recursos

---

## P2 — Qualitat i documentació

### 10. Tests E2E de Playwright ⚠️ PARCIAL

Fitxers creats però amb cobertura incompleta:

- [x] `frontend/tests/e2e/encryption/none.spec.mjs` — ✅ complet
- [x] `frontend/tests/e2e/encryption/symmetric.spec.mjs` — ✅ complet
- [x] `frontend/tests/e2e/encryption/asymmetric.spec.mjs` — ✅ complet
- [ ] `frontend/tests/e2e/servers.spec.mjs` — esbós bàsic, cal ampliar
- [ ] `frontend/tests/e2e/channels.spec.mjs` — esbós bàsic, cal ampliar
- [ ] `frontend/tests/e2e/messages.spec.mjs` — esbós bàsic, cal ampliar
- [ ] `frontend/tests/e2e/friends.spec.mjs` — esbós bàsic, cal ampliar
- [ ] `frontend/tests/e2e/permissions.spec.mjs` — esbós bàsic, cal ampliar
- [ ] `frontend/tests/e2e/voice.spec.mjs` — NO existeix
- [ ] `frontend/tests/e2e/dm.spec.mjs` — NO existeix
- [ ] `frontend/tests/e2e/server-invitation.spec.mjs` — NO existeix

---

### 11. Tests unitaris Rust ✅ COMPLET

Els tests d'integració (`channel_flow_integration.rs`, `crypto_flow_integration.rs`) i els unitaris inline:

- [x] Tests unitaris crypto — inline als fitxers font (idiomàtic Rust): `kyber.rs` (5 tests), `aes_gcm.rs` (12 tests), `hash.rs` (7 tests)

---

### 12. Actualitzar documentació ⚠️ PARCIAL

- [x] `definitions/API.md` — leave server + server invitations ja documentats
- [ ] `definitions/OVERVIEW.md` — afegir taula de límits: S3 (storage/transfer) i streaming LiveKit
- [ ] `definitions/DEVELOPMENT.md` — documentar smoke test de docker-compose.minimal.yml
- [ ] `docs-site/` — sincronitzar amb `definitions/` (canals `ca` i `en`)

---

## Referència ràpida

| Prioritat | Feature | Backend | Frontend | Tests |
|-----------|---------|---------|----------|-------|
| P0 | Leave server | ✅ | ✅ | ✅ |
| P0 | Server invitation accept | ✅ | ✅ | ⚠️ falta E2E |
| P0 | Docker minimal | ✅ | — | ⚠️ falta smoke doc |
| P0 | MAX_FILE_SIZE | ✅ | — | ✅ |
| P0 | Quotes S3 (model + enforcement) | ✅ | — | ✅ |
| P1 | DM v2 complet | ✅ | ⚠️ falta modal TTL | ⚠️ falta E2E |
| P1 | Quotes LiveKit | ✅ | — | ✅ |
| P1 | Notificacions de quota | ✅ | ⚠️ falta bloqueig 100% | ✅ |
| P1 | Pàgina subscripció usuari | — | ❌ | ❌ |
| P2 | Tests E2E Playwright | — | ⚠️ parcial | — |
| P2 | Tests unitaris Rust crypto | ✅ | — | ✅ inline |
| P2 | Actualitzar docs | ⚠️ parcial | — | — |
