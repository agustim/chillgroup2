# ChillGroup v2 — Visió General

## Què és

ChillGroup és una plataforma de comunicació en temps real (text, àudio i vídeo) amb canals de veu i text, inspirada en Discord però amb un enfocament fort en la privadesa i la seguretat criptogràfica.

També inclou gestió persistent d'amics, cerca global d'usuaris i presència en temps real per saber qui està actiu.

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
- **Object storage**: S3-compatible (RustFS en dev) amb upload via URL signada

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

### Usuari Normal
```
1. Registre/Login → genera parella de claus (Kyber-1024)
   (Si OPEN_REGISTER=false, només els admins poden crear usuaris)
2. Configuració/desbloqueig de clau local del dispositiu (vault local)
3. Crea un Server (espai de treball)
4. Crea Canals (text/veio) amb tipus de criptografia
5. Convida membres (per username o link)
6. Gestiona una llista d'amics persistent i cerca usuaris de tota l'eina
7. Xat en temps real (missatges encriptats segons nivell del canal)
8. Voces/Video via LiveKit (E2EE amb session keys)
```

### Administrador (si `OPEN_REGISTER=false`)
```
1. Inici de sessió amb credencials d'admin (ADMIN_USER, ADMIN_PASSWORD)
2. Pot crear/modificar/esborrar usuaris
3. Pot visualitzar llistat de tots els usuaris del sistema
4. Els usuaris creats per admin tindran rol "user" o "admin"
5. Rest de funcionalitats com usuari normal
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
| Auto-destrució missatges | ✅ TTL | ❌ (només amb bots/automatització) | ✅ | ✅ |

## Modes d'Operació

ChillGroup pot funcionar en dos modes depenent de la configuració:

### 1️⃣ Mode Obert (`OPEN_REGISTER=true`)
- Els usuaris es poden registrar lliurement sense restriccions
- No hi ha rol d'administrador
- Ideal per a **comunitats públiques** (forums públics, grups d'interesse)
- Sense control central d'usuaris

### 2️⃣ Mode Restringit (`OPEN_REGISTER=false`)
- Només els administradors poden crear usuaris
- L'endpoint de registre públic és desactivat
- Ideal per a **empreses**, **grups privats** o **instàncies corporatives**
- Requereix credencials d'admin inicial (`ADMIN_USER`, `ADMIN_PASSWORD`)
- Opcionalment es pot promocionar un únic registre via `ONE_ADMIN_INVITATION` + `admin_invitation_code`
- Els admins tenen accés a: crear/modificar/esborrar usuaris, llistar usuaris

**Important**: Els administradors **NO** podem accedir a missatges xifrats (E2EE). Mantenen la privacesa completa.

## Sistema de Plans (SaaS)

ChillGroup implementa un model de **SaaS escalable** amb 3 tiers estàndards + capacitat de customització:

### Plans Predefinits

| Plan | Descripció | Casos d'ús |
|------|-----------|-----------|
| **Free** | 1 servidor, 3 canals text, 2 veu, 20 members | Usuaris individuals, proves, comunitats petites |
| **Pro** | 5 servidors, 20 canals text, 10 veu, 500 members | Grups de treball, empreses petites-mitjes |
| **Enterprise** | Unlimited servidors, canals, members | Grans organitzacions, desplegaments corporatius |

### Features de Limits

**Els límits es verifiquen en "hard mode"** — Si assoliu el límit, no pots crear més recursos:
- ✅ Servidors per usuari
- ✅ Canals de text per servidor
- ✅ Canals de veu per servidor  
- ✅ Members per servidor
- ✅ API calls per minut (rate limiting)
- ✅ Missatges per dia

Els **admins poden canviar el plan de cada usuari** en temps real, sense perdre dades existents.

