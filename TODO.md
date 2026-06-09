# TODO

> **Última revisió:** 2026-06-09

## Context

La capa base de plans/tiers ja existeix (models, endpoints, límits hard i UI admin). Aquest TODO és per ampliar-la.

## P0 (prioritat alta) — ✅ TOT IMPLEMENTAT

* ~~Docker mínim (només LiveKit + S3) per desenvolupar/provar localment.~~ ✅
* ~~Afegir variable d'entorn `MAX_FILE_SIZE` per limitar la mida màxima per fitxer en upload.~~ ✅
* ~~Estendre els plans actuals amb quotes de fitxers (S3):~~ ✅
    * Free: 10 GB espai, 100 GB transferència mensual.
    * Pro: 50 GB espai, 500 GB transferència mensual.
    * Enterprise: 200 GB espai, transferència il·limitada.
* ~~Persistir i calcular consum mensual de fitxers (espai i transferència) per usuari.~~ ✅
* ~~Aplicar enforcement als endpoints d'adjunts (bloqueig quan se supera el límit).~~ ✅

## P1 (prioritat mitjana) — ✅ BACKEND COMPLET / ⚠️ Frontend parcial

* ~~Estendre els plans amb quotes d'audio/video (LiveKit):~~ ✅
    * Free: 10 hores de streaming mensual.
    * Pro: 50 hores de streaming mensual.
    * Enterprise: il·limitades.
* ~~Persistir i calcular consum mensual de streaming per usuari.~~ ✅
* ~~Implementar notificacions de límit de pla (avisos al 80/90%).~~ ✅ backend + frontend (AppLayout.tsx) / Bloqueig UI al 100% ❌ pendent
* Crear pàgina d'autoservei de subscripció per a usuaris (veure pla actual + flux de canvi de pla). ❌

## P2 (documentació i qualitat) — ⚠️ PARCIAL

* Actualitzar documentació a `definitions/` i `docs-site/` amb els nous límits i comportament d'errors. ⚠️ API.md ✅ / Falta: S3/streaming limits a OVERVIEW.md, smoke test a DEVELOPMENT.md
* Afegir tests d'integració per quotes d'S3 i LiveKit (free/pro/enterprise). ✅
* ~~Tests unitaris Rust de crypto (kyber, aes_gcm, channel_keys).~~ ✅ inline als fitxers font (24 tests)
* Tests E2E Playwright complets (servers, channels, messages, DM, invitations). ⚠️ Parcial
