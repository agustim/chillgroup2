# ChillGroup — Cryptographic System

## Summary

ChillGroup supports three security levels per channel. The user selects the level when creating the channel:

| Level | Name | Message encryption | Where channel key lives | Zero-knowledge | Versioning |
|-------|------|-------------------|------------------------|----------------|------------|
| 0 | None | ❌ No | — | ❌ No | ❌ No |
| 1 | Symmetric | ✅ AES-GCM-256 | ✅ Server (encrypted with master key) | ❌ No (server can decrypt) | ✅ Yes |
| 2 | Asymmetric | ✅ AES-GCM-256 | ❌ Never on server | ✅ Yes (100%) | ✅ Yes |

Audio and video use **LiveKit E2EE** with session keys independent of the chat.

## Local Key Protection (Client Vault)

Since June 2026, channel keys stored on the client are not saved in plaintext in IndexedDB when the device has local vault enabled.

### Goal

Prevent an attacker with access to the browser profile from reading keys directly from IndexedDB at rest.

### Model

1. Normal login (`username + password`) to authenticate with the server.
2. After login, the client requests a **local unlock key** for the device:
   1. if it is the first time on that browser, it is created,
   2. if a vault already exists, it must be unlocked.
3. With the vault unlocked, channel keys are stored encrypted at rest (`keyCiphertext`) with AES-GCM.
4. The local key is never sent to the server.

### Logout and local persistence

On logout, the user can independently choose to:

1. make a backup (encrypted or not),
2. delete local data or keep it encrypted.

If local data is kept, the next login will require a local unlock to use it.

### Local key rotation

The UI allows changing the local unlock key. During this process:

1. the current local key is validated,
2. a new local key is created,
3. local channel keys are re-encrypted with the new key.

---

## Key Sharing (Current Implementation)

This section describes when and how keys are shared in the real system (frontend + backend) for each scenario.

### 1) Symmetric Channel (level 1)

#### 1.1 Channel creation

1. The client creates the channel with `encryption_type = symmetric`.
2. The server generates a `channel_key` AES-256.
3. The server stores it in `channel_key_versions` encrypted with `server_master_key`.

#### 1.2 First access / key retrieval

1. The client enters the channel and calls `ensureChannelKey(...)`.
2. If it has no local key, it calls `GET /api/channels/{channel_id}/keys`.
3. The server validates channel access:
   1. If the channel is private, the user must be a channel member.
   2. If public, being a server member is enough.
4. The server decrypts the `channel_key` (with `server_master_key`), encapsulates it for the requesting device (ML-KEM), and returns `encryptedKey + kemCiphertext`.
5. The client decapsulates, obtains the key and stores it locally (encrypted at rest if local vault is active).

**Key point**: in symmetric mode, there is no key "push" between users; each authorised device requests the key from the server when needed.

### 2) Asymmetric Channel (level 2)

#### 2.1 Channel creation

1. The server creates the channel and the initial version (`channel_key_versions`, version 1) without knowing the content key.
2. The creator client generates a `channel_key` AES-256 locally.
3. The client queries `GET /api/channels/{channel_id}/member-devices`.
4. For each returned device:
   1. Encapsulate with ML-KEM (`kemPublicKey` of the recipient).
   2. Encrypt the `channel_key` with the derived shared secret.
   3. Sign the bundle with the signing device's ML-DSA key.
5. The client uploads bundles via `POST /api/channels/{channel_id}/keys`.

#### 2.2 Member access without the key

1. The client calls `GET /api/channels/{channel_id}/keys`.
2. The server returns the bundle stored for the JWT's `device_id`.
3. The client validates the bundle signature.
4. If valid, decapsulates and recovers the `channel_key`.

**Key point**: the server cannot reconstruct the `channel_key`; it only stores per-device encrypted bundles.

### 3) Inviting a user to a channel

#### 3.1 Public channel (`is_private = false`)

1. The inviting user calls `POST /api/channels/{channel_id}/invite`.
2. The backend ensures the invitee is a server member.
3. The inviter redistributes the channel key (asymmetric) by uploading bundles for the invitee's devices.

#### 3.2 Private channel (`is_private = true`)

1. The creator is automatically added to `channel_members` when the channel is created.
2. On `POST /api/channels/{channel_id}/invite`, the backend adds the invitee to `channel_members`.
3. Only after that can the invitee:
   1. see the channel,
   2. request keys,
   3. see messages,
   4. appear in `member-devices`.

**Key point**: in private channels, key sharing and access are governed by channel members, not by all server members.

### 4) New device for an existing user

#### 4.1 Symmetric

1. The new device registers its public keys.
2. When entering a channel, it requests the key from the server (`GET /keys`) and retrieves it.

#### 4.2 Asymmetric

1. The new device publishes `kemPublicKey` + `dsaPublicKey`.
2. A member who has the `channel_key` needs to redistribute bundles for this device.
3. This happens automatically at points such as:
   1. opening the channel (best-effort redistribution),
   2. inviting to a channel,
   3. inviting to a server (redistribution of known asymmetric channels with local key available).

### 5) Exact points where keys are shared

1. Creating an asymmetric channel: the creator generates and uploads initial bundles.
2. Opening an asymmetric channel: best-effort redistribution if the client has the local key.
3. Inviting a user to a channel: explicit redistribution after the `invite`.
4. Inviting a user to a server: redistribution for asymmetric channels where a local key is available.

### 6) Access rules (summary)

1. Public channel:
   - server member = can access the channel.
2. Private channel:
   - channel member = can access the channel,
   - non-channel member = cannot see channel, cannot receive key, cannot see messages.
