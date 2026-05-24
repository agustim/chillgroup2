# ChillGroup v2 — Guia de Documentació

## Què és

ChillGroup és una plataforma de comunicació en temps real (text, àudio i vídeo) amb tres nivells de seguretat criptogràfica: sense xifratge, clau simètrica, i clau asimètrica E2EE (quantum-resistant amb Kyber-1024).

**Stack**: Rust (server) + TypeScript (frontend) + LiveKit (àudio/vídeo) + SQLx (BD).

Aquesta carpeta conté **tota la especificació tècnica** necessària per construir el projecte des de zero, incloent-hi un agent LLM.

## Documents

| Fitxer | Línies | Què explica |
|--------|--------|-------------|
| [OVERVIEW.md](OVERVIEW.md) | 71 | Visió general, stack, principis, comparatives |
| [ARCHITECTURE.md](ARCHITECTURE.md) | 549 | Arquitectura Rust/Axum, capes, LiveKit, SQLx |
| [CRYPTOGRAPHY.md](CRYPTOGRAPHY.md) | 677 | 3 nivells (none/symmetric/asymmetric), Kyber-1024, fluxos KEM |
| [DATABASE.md](DATABASE.md) | 536 | Migrations SQL, models Rust, multi-DB (PostgreSQL/SQLite) |
| [DEVELOPMENT.md](DEVELOPMENT.md) | 834 | Pla de 7 fases, workspace Cargo, Docker |
| [FRONTEND.md](FRONTEND.md) | 1077 | Layout, CSS variables, temes, fonts, storage, components |
| [TESTING.md](TESTING.md) | 1264 | TDD, Playwright E2E, fixtures, escenaris, CI/CD |
| [API.md](API.md) | 951 | 27 endpoints amb request/response JSON exactes, incloent amics i cerca global |
| [DM.md](DM.md) | 205 | Disseny objectiu de DM 1:1 asimètric amb TTL, API i migració |
| [SOCKET.md](SOCKET.md) | 708 | 18 events, payloads, rooms, protocol temps real, incloent presència d'amics |
| [ERRORS.md](ERRORS.md) | 825 | 53 codis d'error, Rust type, TypeScript handling |

## Com Llegir

### Per a un desenvolupador que comença

1. **OVERVIEW.md** → Què estic construint?
2. **ARCHITECTURE.md** → Com està estructurat?
3. **DEVELOPMENT.md** → Per on començo?
4. **API.md** → Quines són les interfícies?
5. **DM.md** → Contracte objectiu de missatges directes 1:1
6. **SOCKET.md** → Com funciona el temps real?
7. **CRYPTOGRAPHY.md** → Com funciona la seguretat?

### Per a un agent LLM que ha de construir el projecte

1. Llegir tots els documents en ordre (OVERVIEW → ERRORS)
2. Començar per la **Fase 1** de DEVELOPMENT.md (infraestructura base)
3. Seguir cada fase en ordre, implementant **tests abans de codi** (TESTING.md)
4. Consultar **API.md** per a cada endpoint
5. Consultar **SOCKET.md** per a cada event de temps real
6. Consultar **ERRORS.md** per a cada cas d'error
7. Consultar **FRONTEND.md** per a cada component de UI

### Per a un revisor de codi

1. **ARCHITECTURE.md** → El codi segueix el pattern de capes?
2. **API.md** → L'endpoint segueix el contracte definit?
3. **ERRORS.md** → S'estan retornant els codis d'error correctes?
4. **TESTING.md** → Hi ha tests per a les funcionalitats noves?
5. **CRYPTOGRAPHY.md** → La implementació criptogràfica segueix els fluxos definits?

## Relació entre Documents

```
OVERVIEW
    ↓
ARCHITECTURE ─── DATABASE ─── CRYPTOGRAPHY
    ↓
DEVELOPMENT ─── FRONTEND
    ↓              ↓
  TESTING ─── API ─── SOCKET ─── ERRORS
                            │
                            └── DM
```

- **OVERVIEW** → Context i objectiu
- **ARCHITECTURE** → Estructura general del sistema
- **DATABASE** → Dades persistents i migrations
- **CRYPTOGRAPHY** → Sistema de seguretat
- **DEVELOPMENT** → Pla d'execució pas a pas
- **FRONTEND** → Interfície d'usuari
- **TESTING** → Estratègia de verificació
- **API** → Contracte HTTP
- **DM** → Disseny DM 1:1 amb E2EE asimètric i TTL
- **SOCKET** → Protocol de temps real
- **ERRORS** → Gestió d'errors

## Accions Ràpides

```bash
# Arrancar infraestructura
docker compose up -d postgres livekit

# Executar migracions
cd server && sqlx migrate run && cd ..

# Arrancar servidor Rust
cd server && cargo run

# Arrancar frontend (altre terminal)
cd frontend && npm run dev

# Executar tests
npm test

# Només tests d'encriptació
npm run test:e2e:encryption

# Mode interactiu (veure el browser)
npm run test:headed
```
