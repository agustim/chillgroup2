# ChillGroup v2 — Sistema Criptogràfic

## Resum

ChillGroup suporta tres nivells de seguretat per canal. L'usuari tria el nivell en crear el canal:

| Nivell | Nom | Xifratge missatges | Xifratge clau de canal | Zero-knowledge |
|--------|-----|---------------------|------------------------|----------------|
| 0 | Sense | ❌ No | — | ❌ No |
| 1 | Simètric | ✅ AES-GCM-256 | ✅ AES-256-GCM (clau servidor) | ❌ No (servidor pot llegir) |
| 2 | Asimètric | ✅ AES-GCM-256 | ✅ Kyber-1024 (ML-KEM-1024) + AES-GCM | ✅ Sí |

Àudio i vídeo fan servir **LiveKit E2EE** amb session keys independents del xat.

## Nivell 0 — Sense Criptografia

### Ús
Canals públics, announcements, mems, qualsevol espai on la privadesa no importa.

### Flux

```
1. Client envia missatge en text pla
2. Servidor guarda text pla a la DB
3. Servidor broadcast via Socket.IO en text pla
```

### Seguretat
- Cap protecció contra interceptació de xarxa
- Servidor pot llegir tots els missatges
- Ideal per a comunicació oberta
- Encara es beneficia de TLS en transport

### Base de Dades

```sql
INSERT INTO messages (channel_id, sender_user_id, sender_device_id,
                      encrypted_payload, iv, timestamp)
VALUES ($1, $2, $3, $4, $5, now());
```

`encrypted_payload` conté el text literal. `iv` és NULL.

## Nivell 1 — Clau Simètrica (AES-GCM-256)

### Concepte

Un sol **canal key** (AES-256) es compartit per tots els membres del canal. La clau es guarda al servidor **xifrada amb una clau mestra del servidor**.

### Generació de Clau Mestra del Servidor

```rust
// Server master key es genera una vegada i es guarda en:
// 1. Variable d'entorn SERVER_MASTER_KEY (hex, 64 bytes = 256 bits)
// 2. O en un fitxer local: ~/.chillgroup/master_key.hex
// 3. O en AWS Secrets Manager / HashiCorp Vault (prod)

pub struct ServerMasterKey {
    key: Aes256Key,  // 32 bytes
}

impl ServerMasterKey {
    pub fn from_env() -> Result<Self> {
        let hex_str = std::env::var("SERVER_MASTER_KEY")?;
        let key = hex::decode(&hex_str)?.try_into()?;
        Ok(Self { key })
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<(Vec<u8>, AesGcmNonce)> {
        let nonce = AesGcmNonce::generate();
        let cipher = Aes256Gcm::new(&self.key);
        let ciphertext = cipher.encrypt(&nonce, data)?;
        Ok((ciphertext, nonce))
    }

    pub fn decrypt(&self, ciphertext: &[u8], nonce: &AesGcmNonce) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(&self.key);
        cipher.decrypt(nonce, ciphertext)
    }
}
```

### Flux de Creació de Canal

```rust
pub async fn create_symmetric_channel(
    channel_id: Uuid,
    creator_device_id: Uuid,
    master_key: &ServerMasterKey,
) -> Result<ChannelKeyRecord> {
    // 1. Generar clau aleatòria de canal
    let channel_key = Aes256Key::generate();  // 32 bytes aleatoris
    let channel_key_bytes = channel_key.as_ref();

    // 2. Encriptar amb la clau mestra del servidor
    let (encrypted_key, nonce) = master_key.encrypt(channel_key_bytes)?;

    // 3. Guardar al servidor
    db.channel_keys().insert(ChannelKeyRecord {
        channel_id,
        device_id: creator_device_id,
        encrypted_key: base64_encode(&encrypted_key),
        nonce: base64_encode(nonce.as_ref()),
        encryption_type: "symmetric",
    }).await?;

    // 4. Retornar al client (el creador guarda la clau en clar a IndexedDB)
    Ok(ChannelKeyResponse { channel_key })
}
```

### Flux d'Accés (Membre)

```rust
pub async fn get_channel_key(
    channel_id: Uuid,
    device_id: Uuid,
    master_key: &ServerMasterKey,
) -> Result<Aes256Key> {
    // 1. Obtenir la clau encriptada del servidor
    let record = db.channel_keys().get_for_device(channel_id, device_id).await?;

    // 2. Desencriptar amb la clau mestra
    let encrypted = base64_decode(&record.encrypted_key)?;
    let nonce = AesGcmNonce::from_slice(&base64_decode(&record.nonce)?);
    let decrypted = master_key.decrypt(&encrypted, &nonce)?;

    // 3. Retornar la clau en clar (només per breus segons)
    Ok(Aes256Key::try_from(decrypted.as_slice())?)
}
```

### Xifratge de Missatges

```rust
pub fn encrypt_message(key: &Aes256Key, plaintext: &str) -> Result<MessagePayload> {
    let plaintext_bytes = plaintext.as_bytes();
    let iv = AesGcmNonce::generate();
    let cipher = Aes256Gcm::new(key);
    let ciphertext = cipher.encrypt(&iv, plaintext_bytes)?;

    Ok(MessagePayload {
        encrypted_data: base64_encode(&ciphertext),
        iv: base64_encode(iv.as_ref()),
    })
}

pub fn decrypt_message(key: &Aes256Key, payload: &MessagePayload) -> Result<String> {
    let ciphertext = base64_decode(&payload.encrypted_data)?;
    let iv = AesGcmNonce::from_slice(&base64_decode(&payload.iv)?);
    let cipher = Aes256Gcm::new(key);
    let plaintext = cipher.decrypt(&iv, &ciphertext)?;
    Ok(String::from_utf8(plaintext)?)
}
```

### Limitacions
- ⚠️ **Servidor pot llegir missatges** (té la master key)
- ⚠️ Si es compromet la master key, **tots els missatges de tots els canals es comprometen**
- ⚠️ No hi ha **forward secrecy** — la mateixa clau dura tot el temps de vida del canal
- ✅ Simple i eficient — cap cost addicional per membre
- ✅ Ideal per equips petits que confien en el servidor

## Nivell 2 — Clau Asimètrica (Kyber-1024 + AES-GCM)

### Concepte

Cada membre rep la seva pròpia còpia de la clau de canal encriptada amb la seva **clau pública Kyber-1024**. El servidor guarda les còpies encriptades però **no pot desxifrar-les**.

### Criptografia Utilitzada

#### Kyber-1024 (ML-KEM-1024)

- **NIVEL DE SEGUREtat**: NIST Level 5 (el més alt)
- **Clau pública**: 1568 bytes
- **Clau privada**: 3168 bytes
- **Ciphertext KEM**: 1088 bytes
- **Shared secret**: 32 bytes (256 bits)
- **Resistent a**: Atacs clàssics i quàntics

```rust
// Dependències de RustCrypto
use x25519_dilithium::{KeyPair, EncapsulationKey, DecapsulationKey};
use rand::rngs::OsRng;

// Generar parella de claus (un cop per dispositiu)
let keypair = KeyPair::generate(&mut OsRng);
let encapsulating = keypair.encapsulating_key();
let decapsulating = keypair.decapsulating_key();

// Public key es guarda al servidor (devices.public_key)
let public_key_bytes: Vec<u8> = (&encapsulating).into();
// 1568 bytes → Base64

// Secret key es guarda LOCALMENT (IndexedDB al frontend)
let secret_key: KeyPair = keypair;
// MAI surt del dispositiu
```

#### Derivació de Claus (HKDF)

```rust
use hkdf::Hkdf;
use hmac::Hmac;
use sha2::Sha256;

type HmacSha256 = Hkdf<Hmac<Sha256>>;

fn derive_kek(shared_secret: &[u8], channel_id: Uuid) -> Aes256Key {
    let hkdf = HmacSha256::new(b"chillgroup-channel-key");
    let mut kek = [0u8; 32];
    hkdf.expand(&channel_id.as_bytes().to_vec(), &mut kek)
        .expect("HKDF should never fail with 32-byte output");
    Aes256Key::from(kek)
}
```

### Flux Complet

#### 1. Registre de Dispositiu (única vegada)

```rust
// Frontend TypeScript
import { MLKEM1024 } from '@noble/post-quantum'

// Generar claus Kyber-1024
const keypair = await MLKEM1024.keygen()

// Guardar secretKey a IndexedDB
await indexedDB.put('keypairs', {
    deviceId: currentDeviceId,
    type: 'kyber',
    secretKey: keypair.secretKey,
    createdAt: Date.now()
})

// Enviar publicKey al servidor
await fetch('/api/user/me/devices/' + deviceId + '/publicKey', {
    method: 'PUT',
    headers: { 'Authorization': 'Bearer ' + token },
    body: JSON.stringify({
        publicKey: btoa(keypair.publicKey)
    })
})
```

#### 2. Creació de Canal Asimètric

```rust
pub async fn create_asymmetric_channel(
    channel_id: Uuid,
    creator_device_id: Uuid,
) -> Result<CreateChannelResult> {
    // 1. Generar clau AES-256 aleatòria per al canal
    let channel_key = Aes256Key::generate();

    // 2. Encapsular amb la pròpia clau Kyber del creador
    let creator_device = db.devices().get(creator_device_id).await?;
    let creator_public_key = decode_base64(&creator_device.public_key)?;

    // KEM encapsulation
    let (shared_secret, ciphertext) = kem_encapsulate(&creator_public_key)?;
    let kek = derive_kek(&shared_secret, channel_id);
    let encrypted_key = aes_gcm_encrypt(&kek, channel_key.as_ref())?;

    // 3. Guardar al servidor
    db.channel_keys().insert(ChannelKeyRecord {
        channel_id,
        device_id: creator_device_id,
        encrypted_key: base64_encode(&encrypted_key),
        kem_ciphertext: base64_encode(&ciphertext),
        encryption_type: "asymmetric",
    }).await?;

    // 4. Retornar clau en clar al creador (per encriptar missatges)
    Ok(CreateChannelResult { channel_key })
}
```

#### 3. Convidar Membre

```rust
pub async fn invite_member_asymmetric(
    channel_id: Uuid,
    target_device_ids: Vec<Uuid>,
) -> Result<Vec<EncryptedChannelKey>> {
    // 1. Obtenir channel_key en clar (del creador, de la memòria o IndexedDB)
    // En producció, això s'hauria de fer al client, no al servidor
    // El servidor només guarda les còpies encriptades

    // 2. Per cada dispositiu objectiu
    let mut results = Vec::new();
    for device_id in target_device_ids {
        let target_device = db.devices().get(device_id).await?;
        let target_public_key = decode_base64(&target_device.public_key)?;

        // KEM encapsulate amb la publicKey del destinataris
        let (shared_secret, ciphertext) = kem_encapsulate(&target_public_key)?;
        let kek = derive_kek(&shared_secret, channel_id);
        let encrypted_key = aes_gcm_encrypt(&kek, channel_key.as_ref())?;

        // Guardar al servidor
        db.channel_keys().upsert(ChannelKeyRecord {
            channel_id,
            device_id,
            encrypted_key: base64_encode(&encrypted_key),
            kem_ciphertext: base64_encode(&ciphertext),
            encryption_type: "asymmetric",
        }).await?;

        results.push(EncryptedChannelKey {
            device_id,
            encrypted_key: base64_encode(&encrypted_key),
            kem_ciphertext: base64_encode(&ciphertext),
        });
    }

    Ok(results)
}
```

**Nota important**: En una implementació real, aquest pas es fa **al client** (frontend), no al servidor. El servidor només rep i guarda les còpies encriptades.

```typescript
// Frontend TypeScript — Convidar membre
async function inviteMember(channelId: string, targetDeviceIds: string[]) {
    // Obtenir channelKey de IndexedDB (el creador la té)
    const channelKey = await indexedDB.getKey('channelKeys', channelId)

    const encryptedKeys = []
    for (const deviceId of targetDeviceIds) {
        // Obtenir publicKey del dispositiu target
        const device = await getDevicePublicKeys(deviceId)
        const publicKey = base64Decode(device.publicKey)

        // KEM encapsulate
        const { sharedSecret, ciphertext } = await kemEncapsulate(publicKey)

        // Derivar KEK
        const kek = await deriveKek(sharedSecret, channelId)

        // Encriptar channelKey amb KEK
        const encryptedKey = await aesGcmEncrypt(kek, channelKey)

        encryptedKeys.push({
            deviceId,
            encryptedKey: btoa(encryptedKey),
            ciphertext: btoa(ciphertext)
        })
    }

    // Enviar al servidor (només dades encriptades)
    await fetch('/api/channels/' + channelId + '/invite', {
        method: 'POST',
        body: JSON.stringify({ encryptedKeys })
    })
}
```

#### 4. Membre Accedeix al Canal (Desencripta)

```typescript
// Frontend TypeScript — Recuperar clau de canal
async function getChannelKey(channelId: string): Promise<CryptoKey> {
    // 1. Obtenir còpia encriptada del servidor
    const records = await fetch('/api/channels/' + channelId + '/keys')

    // 2. Per cada dispositiu, intentar desencriptar
    for (const record of records) {
        const secretKey = await indexedDB.getKey('keypairs', record.deviceId)
        if (!secretKey) continue

        // KEM decapsulate
        const sharedSecret = await kemDecapsulate(
            secretKey,
            base64Decode(record.kemCiphertext)
        )

        // Derivar KEK
        const kek = await deriveKek(sharedSecret, channelId)

        // Desencriptar channelKey
        try {
            const channelKeyBytes = await aesGcmDecrypt(kek, base64Decode(record.encryptedKey))
            // Guardar a IndexedDB per ús futur
            await indexedDB.put('channelKeys', {
                channelId,
                key: channelKeyBytes,
                acquiredAt: Date.now()
            })
            return channelKeyBytes
        } catch {
            continue // Provar següent dispositiu
        }
    }

    throw new Error('No es pot obtenir la clau del canal')
}
```

#### 5. Enviar Missatge

```typescript
async function sendMessage(channelId: string, text: string) {
    // 1. Obtenir clau del canal
    const channelKey = await getChannelKey(channelId)

    // 2. Encriptar amb AES-GCM
    const iv = crypto.getRandomValues(new Uint8Array(12))
    const encoder = new TextEncoder()
    const encrypted = await crypto.subtle.encrypt(
        { name: 'AES-GCM', iv },
        channelKey,
        encoder.encode(text)
    )

    // 3. Enviar al servidor
    await fetch('/api/channels/' + channelId + '/messages', {
        method: 'POST',
        body: JSON.stringify({
            encryptedPayload: btoa(new Uint8Array(encrypted)),
            iv: btoa(new Uint8Array(iv)),
        })
    })
}
```

### Diagrama de Flux Complet

```
CREADOR                                  SERVIDOR                              DESTINATARI
  │                                       │                                     │
  │──[1] Genera AES-256 channelKey ───────│                                     │
  │──[2] KEM.Encrypt(creator.pk) ────────│                                     │
  │       → (sharedSecret, ciphertext)    │                                     │
  │──[3] KEK = HKDF(sharedSecret)        │                                     │
  │──[4] AES.Encrypt(KEK, channelKey)    │                                     │
  │──[5] POST {encryptedKey, ciphertext}─▶│                                     │
  │                                       │──[6] INSERT channel_keys────────    │
  │──[7] POST invite(member.pk) ─────────▶│                                     │
  │                                       │──[8] KEM.Encrypt(member.pk)      │
  │                                       │       → (sharedSecret, ciphertext)│
  │                                       │──[9] AES.Encrypt(KEK, channelKey) │
  │                                       │──[10] INSERT channel_keys────────│
  │                                       │                                     │
  │                                       │  ←────[11] Socket.IO join channel──│
  │                                       │                                     │──[12] GET /keys
  │                                       │──[13] {encryptedKey, ciphertext}──▶│
  │                                       │                                     │──[14] KEM.Decrypt(sk)
  │                                       │                                             → sharedSecret
  │                                       │                                     │──[15] KEK = HKDF(sharedSecret)
  │                                       │                                     │──[16] AES.Decrypt(KEK, encryptedKey)
  │                                       │                                             → channelKey!
  │                                       │                                     │
  │──[17] AES.Encrypt(channelKey, msg)──▶│                                     │
  │                                       │──[18] INSERT message (xifrat)─────│
  │                                       │  ←────[19] Socket.IO message──────│
  │                                       │                                     │──[20] AES.Decrypt(channelKey, msg)
  │                                       │                                             → Text pla!
```

### Limitacions

- ⚠️ **No perfect forward secrecy per missatge** — la mateixa channelKey dura tot el temps de vida del canal
- ⚠️ Si el creador revoca un dispositiu, aquest no pot accedir a nous missatges però sí als històrics (ja encriptats)
- ✅ **Servidor zero-knowledge** — mai pot desxifrar missatges
- ✅ **Per dispositiu** — cada dispositiu té el seu propi keypair Kyber
- ✅ **Quantum-resistant** — Kyber-1024 és ML-KEM-1024 (NIST Level 5)

## Àudio/Vídeo E2EE (LiveKit)

### Session Keys

LiveKit suporta E2EE nativa amb **session keys** independents del sistema de xat:

```typescript
// Frontend — Configurar E2EE a LiveKit
import { Room } from 'livekit-client'

const room = new Room()

// Session key per canal de veu
const sessionKey = await crypto.subtle.generateKey(
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
)

await room.setE2EE(true, {
    key: sessionKey,
    keyStore: new DefaultKeyStore(),  // IndexedDB
})

room.on(RoomEvent.TrackSubscribed, (track, publication, participant) => {
    // Desxifratge automàtic amb la session key
})
```

### Distribució de Session Key

```
Opció A: Via canal de text encriptat (recomanat)
1. Creator genera session key
2. Enviar via canal de text (ja encriptat amb AES o Kyber)
3. Membros desencripten del canal de text → configuren LiveKit E2EE

Opció B: Via handshake manual
1. Creator exporta key como QR code
2. Membre escaneja i introdueix manualment
```

### Servidor Rust — LiveKit Integration

```rust
pub struct LiveKitService {
    client: livekit_server_sdk::Client,
    api_key: String,
    api_secret: String,
}

impl LiveKitService {
    /// Crea una sala de veu i retorna el token per al participant
    pub async fn create_room_and_token(
        &self,
        channel_id: Uuid,
        server_id: Uuid,
        user_id: Uuid,
        display_name: &str,
    ) -> Result<LiveKitToken> {
        let room_name = format!("chill-{}-{}", server_id, channel_id);

        // Verificar permisos
        self.verify_channel_access(channel_id, user_id).await?;

        // Crear sala a LiveKit si no existeix
        self.client.create_room(livekit_server_sdk::CreateRoomRequest {
            name: room_name.clone(),
            max_participants: 50,
            empty_timeout: 900,  // 15 min sense participants
            max_disconnection: 120,
        }).await?;

        // Generar token d'accés
        let token = self.client.create_token()
            .room_name(&room_name)
            .identity(user_id.to_string())
            .display_name(display_name)
            .can_publish(true)
            .can_subscribe(true)
            .can_publish_data(true)
            .sign(&self.api_key, &self.api_secret)?;

        Ok(LiveKitToken {
            token: token.claims().clone().jwt,
            room: room_name,
            e2ee_enabled: true,
        })
    }
}
```

## Detall Tècnic: ML-KEM-1024 (Kyber-1024)

### Paràmetres

| Paràmetre | Valor |
|-----------|-------|
| Nom oficial | ML-KEM-1024 |
| NIVEL NIST | Level 5 (màxim) |
| Seguretat CPI | 256 bits |
| Seguretat CCA2 | 256 bits |
| Clau pública | 1568 bytes |
| Clau privada | 3168 bytes |
| Ciphertext KEM | 1088 bytes |
| Shared secret | 32 bytes (256 bits) |

### Operacions

```rust
// Generació de claus
fn keygen() -> (PublicKey, SecretKey)

// Encapsulació (xifrar una clau compartida)
fn encapsulate(public_key: &PublicKey) -> (SharedSecret, Ciphertext)

// Desencapsulació (recuperar la clau compartida)
fn decapsulate(secret_key: &SecretKey, ciphertext: &Ciphertext) -> SharedSecret

// Verificació (opcional)
fn verify_encapsulation(public_key: &PublicKey, shared_secret: &SharedSecret, ciphertext: &Ciphertext) -> bool
```

### Interoperabilitat

```rust
// El mateix algoritme a Rust i TypeScript
// Rust: x25519-dilithium (RustCrypto)
// TypeScript: @noble/post-quantum o @oasis-protocol/p256

// Les claus es serialitzen a Base64 per transmissió
let pk_base64 = base64_encode(pk_bytes)   // Rust → servidor
let pk_bytes = base64_decode(pk_base64)    // servidor → TypeScript
```

## Rotació de Claus

### Rotació de Clau de Canal

Per millorar la seguretat, es pot implementar la rotació periòdica:

```rust
pub async fn rotate_channel_key(
    channel_id: Uuid,
    old_channel_key: &Aes256Key,
    devices: Vec<Uuid>,
) -> Result<()> {
    // 1. Generar nova clau de canal
    let new_channel_key = Aes256Key::generate();

    // 2. Per cada dispositiu, reencapsular amb la nova clau
    for device_id in devices {
        let device = db.devices().get(device_id).await?;
        let pk = decode_base64(&device.public_key)?;
        let (shared_secret, ciphertext) = kem_encapsulate(&pk)?;
        let kek = derive_kek(&shared_secret, channel_id);
        let encrypted_new_key = aes_gcm_encrypt(&kek, new_channel_key.as_ref())?;

        db.channel_keys().upsert(ChannelKeyRecord {
            channel_id,
            device_id,
            encrypted_key: base64_encode(&encrypted_new_key),
            kem_ciphertext: base64_encode(&ciphertext),
            encryption_type: "asymmetric",
        }).await?;
    }

    // 3. Invalidar canals de missatges anteriors (opcional)
    // Aquesta és la part complexa: cal reenviar tot l'historial amb la nova clau
    // O bé acceptar que l'historial anterior queda amb la clau antiga
}
```

### Rotació de Dispositiu

Quan un dispositiu es perd o es compromèt:

```rust
pub async fn revoke_device(
    device_id: Uuid,
    channel_keys: Vec<Uuid>,
) -> Result<()> {
    // 1. Marcar dispositiu com a revocat
    db.devices().revoke(device_id).await?;

    // 2. Per cada canal amb accesse, reenviar la clau amb un altre dispositiu
    for channel_id in channel_keys {
        let other_devices = db.devices().get_active_for_user(device_id).await?;
        if let Some(replacement_device) = other_devices.first() {
            // Re-encapsular amb el dispositiu de reemplaçament
            self.invite_member_asymmetric(channel_id, vec![replacement_device.id]).await?;
        }
    }
}
```

## Resum de Seguretat

| Amenaça | Nivell 0 | Nivell 1 | Nivell 2 |
|---------|----------|----------|----------|
| Eavesdropper de xarxa | ❌ Llegeix | ❌ Llegeix | ✅ No pot llegir |
| Servidor compromès | ❌ Llegeix | ❌ Llegeix | ✅ No pot llegir |
| Atac quàntic | ❌ Vulnerable | ❌ Vulnerable (AES) | ✅ Resistent (Kyber) |
| AES trencat | ❌ Vulnerable | ❌ Vulnerable | ⚠️ Canal vulnerable, clau segura |
| Forward secrecy | ❌ No | ❌ No | ⚠️ Per dispositiu |
| Compromís d'un membre | ⚠️ Total | ⚠️ Total | ⚠️ Total (sense rotació) |

> **Nota**: AES-256 és considerat segur contra atacs quàntics (Grover's algoritme redueix a 128 bits efectius, que és segur). El Kyber-1024 protegeix el procés de negociació de claus.
