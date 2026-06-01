# ChillGroup — Overview

## What is it?

ChillGroup is a real-time communication platform (text, audio and video) with voice and text channels, inspired by Discord but with a strong focus on privacy and cryptographic security.

It also includes persistent friend management, global user search and real-time presence to know who is active.

## Goal

Build a modern, quantum-resistant chat tool with three selectable security levels per channel:

| Level | Name | Description |
|-------|------|-------------|
| 0 | **No encryption** | Plaintext messages. Like public Discord. Ideal for open channels where privacy is not needed. |
| 1 | **Symmetric key** | An AES-256 key is stored on the server (encrypted with the server key). All channel members share the same key to encrypt/decrypt. |
| 2 | **Asymmetric key** | A channel AES-256 key is encrypted with each member's public key (Kyber-1024) and stored on the server in encrypted form. Only the owner of each private key can decrypt the channel key. True E2EE. |

## Technology Stack

### Frontend
- **Framework**: React + TypeScript + Vite
- **Real-time**: Socket.IO (messages, presence, signalling)
- **Audio/Video**: LiveKit client (with native E2EE via Session Keys)
- **UI**: Tailwind CSS or own system with CSS variables

### Backend (Server)
- **Language**: Rust 1.75+
- **Web framework**: Axum + Tokio
- **Real-time**: Socket.IO server (via `socketioxide`) or native Axum WS
- **WebRTC SFU**: LiveKit server SDK (integration with external LiveKit instance)
- **Authentication**: JWT (RS256) + OAuth2

### Database
- **Primary**: PostgreSQL 16 (production)
- **Secondary**: SQLite 3 (development / small instances)
- **Abstraction**: `sqlx` with compile-time checking + pluggable repository layer
- **Migrations**: `sqlx migrate`

### Infrastructure
- **LiveKit**: External instance for audio/video (E2EE with session keys)
- **Deployment**: Docker Compose (dev), Kubernetes or Docker Swarm (prod)
- **Cache**: Optional Redis for active sessions / presence

## Main User Flow

### Normal User
```
1. Register/Login → generates key pair (Kyber-1024)
   (If OPEN_REGISTER=false, only admins can create users)
2. Set up / unlock local device key (local vault)
3. Create a Server (workspace)
4. Create Channels (text/voice) with encryption type
5. Invite members (by username or link)
6. Manage a persistent friends list and search users across the platform
7. Real-time chat (messages encrypted according to channel level)
8. Voice/Video via LiveKit (E2EE with session keys)
```

### Administrator (if `OPEN_REGISTER=false`)
```
1. Login with admin credentials (ADMIN_USER, ADMIN_PASSWORD)
2. Can create/modify/delete users
3. Can view the full user list of the system
4. Users created by admin get role "user" or "admin"
5. All other features same as normal user
```

## Design Principles

1. **Privacy by Default** — When a cryptographic option exists, the most secure option is the default
2. **Zero Knowledge** — The server CANNOT decrypt messages from asymmetric channels
3. **Pluggable Storage** — The same business logic with different database backends
4. **Quantum-Resistant** — Kyber-1024 (ML-KEM-1024 NIST Level 5) for KEM
5. **Modular** — Each component (authentication, messaging, voice) is independent

## Comparison with Alternatives

| Feature | ChillGroup v2 | Discord | Element/Matrix | Telegram |
|---------|---------------|---------|----------------|----------|
| Audio/Video E2EE | ✅ (LiveKit) | ❌ | ❌ | ✅ (no group E2EE) |
| Message E2EE | ✅ (3 levels) | ❌ | ✅ (private only) | ✅ (private only) |
| Quantum-resistant | ✅ Kyber-1024 | ❌ | ❌ | ❌ |
| Open Source | ✅ | ❌ | ✅ | ❌ |
| Message TTL | ✅ TTL | ✅ (bots/automation) | ✅ | ✅ |

## Operation Modes

ChillGroup can operate in two modes depending on configuration:

### 1️⃣ Open Mode (`OPEN_REGISTER=true`)
- Users can register freely without restrictions
- No administrator role
- Ideal for **public communities** (public forums, interest groups)
- No central user control

### 2️⃣ Restricted Mode (`OPEN_REGISTER=false`)
- Only administrators can create users
- The public registration endpoint is disabled
- Ideal for **companies**, **private groups** or **corporate instances**
