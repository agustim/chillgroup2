# ChillGroup v2 — Visió General

## Què és

ChillGroup és una plataforma de comunicació en temps real (text, àudio i vídeo) amb canals de veu i text, inspirada en Discord però amb un enfocament fort en la privadesa i la seguretat criptogràfica.

## Objectiu

Crear una eina de xat moderna, quantum-resistent, amb tres nivells de seguretat escollibles per canal:

| Nivell | Nom | Descripció |
|--------|-----|------------|
| 0 | **Sense criptografia** | Missatges en clar. Com Discord públic. Ideal per canals oberts on la privadesa no és necessària. |
| 1 | **Clau simètrica** | Una clau AES-256 es guarda al servidor (encriptada amb la clau del servidor). Tots els membres del canal comparteixen la mateixa clau per xifrar/desxifrar. |
| 2 | **Clau asimètrica** | Una clau de canal AES-256 es xifra amb les claus públiques dels membres (Kyber-1024) i es guarda al servidor en format encriptat. Només el propietari de cada clau privada pot desxifrar la clau de canal. E2EE veritable. |

## Stack Tecnològic

### Frontend
- **Framework**: React + TypeScript + Vite
- **Comunicació temps real**: Socket.IO (missatges, presència, senyalització)
- **Àudio/Vídeo**: LiveKit client (amb E2EE nativa via Session Keys)
- **UI**: Tailwind CSS o sistema propi amb variables CSS

### Backend (Server)
- **Llenguatge**: Rust 1.75+
- **Web framework**: Axum + Tokio
- **Temps real**: Socket.IO server (via `socketioxide`) o Axum WS natiu
- **WebRTC SFU**: LiveKit server SDK (integració amb instància LiveKit externa)
- **Autenticació**: JWT (RS256) + OAuth2
- **File system**: Actix-multipart per uploads

### Base de Dades
- **Primari**: PostgreSQL 16 (producció)
- **Secundari**: SQLite 3 (desenvolupament / instàncies petites)
- **Abstracció**: `sqlx` amb compile-time checking + layer de repositori pluggable
- **Migracions**: `sqlx migrate`

### Infraestructura
- **LiveKit**: Instància externa per àudio/vídeo (E2EE amb session keys)
- **Desplegament**: Docker Compose (dev), Kubernetes o Docker Swarm (prod)
- **Cache**: Redis opcional per sessions actives / preences

## Fluix d'Usuari Principal

```
1. Registre/Login → genera parella de claus (Kyber-1024)
2. Crea un Server (espai de treball)
3. Crea Canals (text/veio) amb tipus de criptografia
4. Convida membres (per username o link)
5. Xat en temps real (missatges encriptats segons nivell del canal)
6. Voces/Video via LiveKit (E2EE amb session keys)
```

## Principis de Disseny

1. **Privacy by Default** — Quan hi ha opció criptogràfica, per defecte l'opció més segura
2. **Zero Knowledge** — El servidor NO pot desxifrar missatges de canals asimètrics
3. **Pluggable Storage** — La mateixa lògica de negoci amb diferents backends de BD
4. **Quantum-Resistant** — Kyber-1024 (ML-KEM-1024 NIST Level 5) per KEM
5. **Modular** — Cada component (autenticació, missatgeria, veu) és independent

## Comparativa amb Alternatives

| Feature | ChillGroup v2 | Discord | Element/Matrix | Telegram |
|---------|---------------|---------|----------------|----------|
| Àudio/Video E2EE | ✅ (LiveKit) | ❌ | ❌ | ✅ (no E2EE per grup) |
| Missatges E2EE | ✅ (3 nivells) | ❌ | ✅ (només privat) | ✅ (només privat) |
| Quantum-resistant | ✅ Kyber-1024 | ❌ | ❌ | ❌ |
| Open Source | ✅ | ❌ | ✅ | ❌ |
| Auto-destrució missatges | ✅ TTL | ✅ | ✅ | ✅ |
