# ChillGroup v2 — Sistema Criptogràfic

## Resum

ChillGroup suporta tres nivells de seguretat per canal. L'usuari tria el nivell en crear el canal:

| Nivell | Nom | Xifratge missatges | On viu la clau de canal | Zero-knowledge | Versioning |
|--------|-----|---------------------|-------------------------|----------------|------------|
| 0 | Sense | ❌ No | — | ❌ No | ❌ No |
| 1 | Simètric | ✅ AES-GCM-256 | ✅ Servidor (xifrada amb master key) | ❌ No (servidor pot desxifrar) | ✅ Sí |
| 2 | Asimètric | ✅ AES-GCM-256 | ❌ Mai al servidor | ✅ Sí (100%) | ✅ Sí |

Àudio i vídeo fan servir **LiveKit E2EE** amb session keys independents del xat.

## Protecció Local de Claus (Client Vault)

Des de juny 2026, les claus de canal guardades al client no es desen en clar a IndexedDB si el dispositiu té vault local activat.

### Objectiu

Evitar que un atacant amb accés al perfil del navegador pugui llegir les claus directament de IndexedDB en fred.

### Model

1. Inici de sessió normal (`username + contrasenya`) per autenticar al servidor.
2. Després de l'inici de sessió, el client demana **clau local de desbloqueig** del dispositiu:
  1. si és primer cop en aquell navegador, es crea,
  2. si ja existeix vault, cal desbloquejar-lo.
3. Amb el vault desbloquejat, les claus de canal es guarden xifrades en repòs (`keyCiphertext`) amb AES-GCM.
4. La clau local no s'envia al servidor.

### Logout i persistència local

En sortir, l'usuari pot triar independentment:

1. fer backup (xifrat o no),
2. esborrar dades locals o mantenir-les xifrades.

Si manté dades locals, el proper inici de sessió requerirà desbloqueig local per poder-les usar.

### Rotació de clau local

La UI permet canviar la clau local de desbloqueig. En aquest procés:

1. es valida la clau local actual,
2. es crea una nova clau local,
3. es re-xifren les claus de canal locals amb la nova clau.

---

## Compartició de Claus (Implementació Actual)

Aquest apartat descriu quan i com es comparteixen claus al sistema real (frontend + backend), per cada escenari.

### 1) Canal Simètric (nivell 1)

#### 1.1 Creació de canal

1. El client crea el canal amb `encryption_type = symmetric`.
2. El servidor genera una `channel_key` AES-256.
3. El servidor la guarda a `channel_key_versions` xifrada amb `server_master_key`.

#### 1.2 Primer accés / recuperació de clau

1. El client entra al canal i intenta `ensureChannelKey(...)`.
2. Si no té clau local, fa `GET /api/channels/{channel_id}/keys`.
3. El servidor valida accés al canal:
  1. Si el canal es privat, ha de ser membre del canal.
  2. Si es públic, n'hi ha prou amb ser membre del servidor.
4. El servidor desencripta la `channel_key` (amb `server_master_key`), l'encapsula per al dispositiu demanant (ML-KEM), i retorna `encryptedKey + kemCiphertext`.
5. El client decapsula, obté la clau i la guarda localment (xifrada en repòs si el vault local està actiu).

Punt clau: en simètric, no hi ha "push" de claus entre usuaris; cada dispositiu autoritzat la demana al servidor quan la necessita.

### 2) Canal Asimètric (nivell 2)

#### 2.1 Creació de canal

1. El servidor crea el canal i la versió inicial (`channel_key_versions`, version 1) sense conèixer la clau de contingut.
2. El client creador genera localment una `channel_key` AES-256.
3. El client consulta `GET /api/channels/{channel_id}/member-devices`.
4. Per cada dispositiu retornat:
  1. Encapsula amb ML-KEM (`kemPublicKey` destinatari).
  2. Xifra la `channel_key` amb el shared secret derivat.
  3. Signa el bundle amb ML-DSA del dispositiu signant.
5. El client puja bundles via `POST /api/channels/{channel_id}/keys`.

#### 2.2 Accés d'un membre que no té la clau

1. El client fa `GET /api/channels/{channel_id}/keys`.
2. El servidor retorna el bundle guardat per al `device_id` del JWT.
3. El client valida la signatura del bundle.
4. Si es valida, decapsula i recupera la `channel_key`.

Punt clau: el servidor no pot reconstruir la `channel_key`; només guarda bundles xifrats per dispositiu.

### 3) Convidar usuari al canal

#### 3.1 Canal públic (`is_private = false`)

1. L'usuari convidador fa `POST /api/channels/{channel_id}/invite`.
2. El backend garanteix que el convidat és membre del servidor.
3. El convidador redistribueix la clau del canal (asimètric) pujant bundles per als dispositius del convidat.

#### 3.2 Canal privat (`is_private = true`)

1. El creador queda afegit automàticament a `channel_members` en crear el canal.
2. En `POST /api/channels/{channel_id}/invite`, el backend afegeix el convidat a `channel_members`.
3. Només després, el convidat pot:
  1. veure el canal,
  2. demanar claus,
  3. veure missatges,
  4. aparèixer a `member-devices`.

Punt clau: en canals privats, la compartició de claus i l'accés es governen per membres del canal, no per tots els membres del servidor.

### 4) Dispositiu nou d'un usuari existent

#### 4.1 Simètric

1. El dispositiu nou registra les seves claus públiques.
2. Quan entra al canal, demana clau al servidor (`GET /keys`) i la recupera.

#### 4.2 Asimètric

1. El dispositiu nou publica `kemPublicKey` + `dsaPublicKey`.
2. Necessita que algun membre amb `channel_key` redistribueixi bundles per a aquest dispositiu.
3. Això passa automàticament en punts com:
  1. entrar al canal (redistribució best-effort),
  2. convidar a canal,
  3. convidar a servidor (redistribució de canals asimètrics coneguts localment).

### 5) Punts exactes on es comparteixen claus

1. Crear canal asimètric: el creador genera i puja bundles inicials.
2. Obrir canal asimètric: redistribució best-effort si el client té la clau local.
3. Convidar usuari a canal: redistribució explícita després del `invite`.
4. Convidar usuari a servidor: redistribució per canals asimètrics on hi ha clau local disponible.

### 6) Regles d'accés (resum)

1. Canal públic:
  1. membre de servidor = pot accedir al canal.
2. Canal privat:
  1. membre de canal = pot accedir al canal,
  2. no membre de canal = no veu canal, no rep clau, no veu missatges.

### 7) Errors típics de compartició

1. `ChannelKeyNotFound`:
  1. en simètric, no hi ha versió de clau o falta registre de clau pública del dispositiu,
  2. en asimètric, no existeix bundle per aquell `device_id`.
2. Error de redistribució asimètrica:
  1. clau pública de dispositiu invàlida o corrupta,
  2. clau de signatura local invàlida,
  3. dispositiu sense permisos de canal.

Els errors de redistribució s'escriuen a consola per poder diagnosticar dispositiu per dispositiu.

---

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

---

## Nivell 1 — Clau Simètrica (AES-GCM-256)

### Concepte

Cada canal té una o més **versions de clau** (AES-256). La clau viu al servidor, xifrada amb la master key del servidor. El servidor actua com a dipositari de confiança: pot desxifrar la clau de canal si cal, per lliurar-la als dispositius autoritzats.

Quan un client necessita la clau (primer accés o clau perduda), la sol·licita al servidor presentant la seva clau pública ML-KEM. El servidor comprova que el dispositiu té accés, encripta la clau de canal "al vol" amb la clau pública del dispositiu, i la retorna. El client la desencripta localment i la guarda a IndexedDB xifrada en repòs (si el vault local està actiu).

### Versioning de Claus

Cada canal simètric pot tenir múltiples versions de clau al llarg del temps. Quan la clau canvia (rotació manual o expulsió de membre), els missatges anteriors queden vinculats a la versió antiga i els nous a la nova versió.

**Esquema de taules:**

```sql
channel_key_versions
  id              UUID PRIMARY KEY
  channel_id      UUID NOT NULL REFERENCES channels(id)
  version         INTEGER NOT NULL
  encrypted_key   TEXT NOT NULL      -- AES-256-GCM(master_key, channel_key_plaintext)
  nonce           TEXT NOT NULL      -- nonce per desxifrar encrypted_key
  created_at      TIMESTAMP NOT NULL
  created_by      UUID REFERENCES users(id)
  deprecated_at   TIMESTAMP          -- NULL = versió activa
  UNIQUE (channel_id, version)

messages
  id              UUID PRIMARY KEY
  channel_id      UUID NOT NULL
  key_version_id  UUID REFERENCES channel_key_versions(id)  -- quin versió s'ha usat
  encrypted_payload TEXT NOT NULL
  iv              TEXT NOT NULL
  ...
```

> **Regla**: quan el client rep un missatge, usa `key_version_id` per saber quina versió de clau necessita. Si no la té localment, la demana al servidor indicant el `key_version_id`.

### Generació de Clau Mestra del Servidor

La server master key es configura una sola vegada i mai canvia (o molt rarament):

```
1. Variable d'entorn: SERVER_MASTER_KEY (hex, 64 caràcters = 32 bytes = 256 bits)
2. O fitxer local: ~/.chillgroup/master_key.hex
3. O Vault (producció): HashiCorp Vault / AWS Secrets Manager / Azure Key Vault
```

### Flux de Creació de Canal

```
CLIENT (creador)                        SERVIDOR
  │                                       │
  │ POST /api/servers/{id}/channels       │
  │ { name, encryptionType: "symmetric" } ▶│
  │                                       │── Generar AES-256 channel_key (aleatori)
  │                                       │── AES-GCM.Encrypt(master_key, channel_key) → encrypted_key + nonce
  │                                       │── INSERT channel_key_versions (version=1, encrypted_key, nonce)
  │                                       │── INSERT channels (key_version_id = versió 1)
  │◀── { channelId, keyVersionId: "v1" } ─│
```

El servidor genera la clau. El creador no necessita descarregar-la fins que vol enviar o llegir missatges.

### Flux d'Accés a la Clau (Client sense clau local)

Quan un client obre un canal simètric i no té la clau (o la versió correcta) a IndexedDB:

```
CLIENT                                  SERVIDOR
  │                                       │
  │ GET /api/channels/{id}/keys           │
  │   ?version={key_version_id}           │
  │ Header: Authorization: Bearer JWT ───▶│
  │                                       │── Extreure user_id i device_id del JWT
  │                                       │── Comprovar que user_id és membre del servidor
  │                                       │── Buscar device.public_key a devices
  │                                       │   Si public_key és buida → 403 DeviceNoPublicKey
  │                                       │── Buscar channel_key_versions per version_id
  │                                       │── AES-GCM.Decrypt(master_key, encrypted_key) → channel_key
  │                                       │── ML-KEM.Encapsulate(device.public_key)
  │                                       │     → (kem_ciphertext, shared_secret)
  │                                       │── AES-GCM.Encrypt(shared_secret[0..32], channel_key) → wrapped_key
  │                                       │
  │◀── { keyVersionId, version,           │
  │      wrappedKey, kemCiphertext } ─────│
  │                                       │
  │── ML-KEM.Decapsulate(local_sk, kem_ciphertext) → shared_secret
  │── AES-GCM.Decrypt(shared_secret, wrappedKey) → channel_key
  │── IndexedDB.store(channelId, keyVersionId, channel_key_xifrada_en_repos)
  │── Desxifrar missatges
```

**Errors possibles:**
- `403 Forbidden` — el dispositiu no és membre del servidor
- `403 DeviceNoPublicKey` — el dispositiu no té clau pública ML-KEM registrada (cal fer login amb keypair)
- `404 KeyVersionNotFound` — la versió de clau sol·licitada no existeix

### Flux de Rotació de Clau

Quan un admin vol revocar l'accés a futurs missatges (p.ex. expulsar membre):

```
CLIENT (admin)                          SERVIDOR
  │                                       │
  │ POST /api/channels/{id}/keys/rotate ─▶│
  │                                       │── Generar nova AES-256 channel_key
  │                                       │── AES-GCM.Encrypt(master_key, nova_key)
  │                                       │── INSERT channel_key_versions (version=N+1)
  │                                       │── Marcar versió anterior com deprecated_at = NOW()
  │◀── { keyVersionId, version: N+1 } ───│
```

Els clients detecten el canvi quan reben un missatge amb un `key_version_id` desconegut i tornen a demanar-la al servidor.

### Limitacions del Nivell 1

- ⚠️ **Servidor pot llegir missatges** — té la master key i pot desxifrar qualsevol canal
- ⚠️ Si la master key es compromet, tot queda compromès
- ⚠️ No hi ha perfect forward secrecy per missatge
- ✅ Simple — no cal gestió de keypairs als clients (tot automàtic)
- ✅ Recuperació fàcil — el servidor sempre pot lliurar la clau
- ✅ Versioning — rotació de clau possible sense perdre accés a l'historial
- ✅ Multi-dispositiu transparent — qualsevol dispositiu membre pot obtenir la clau

---

## Nivell 2 — Clau Asimètrica (ML-KEM-1024 + ML-DSA-87 + AES-GCM)

### Concepte

La clau de canal **mai existeix al servidor en cap forma desxifrable**. El servidor només emmagatzema còpies xifrades per a cada dispositiu, signades pel dispositiu que les ha generat. El servidor no pot desxifrar-les ni en teoria.

Quan un membre convida algú, **és el client convidant** qui fa tot el treball criptogràfic: obté les claus públiques del convidat, encripta la channel key per a cada dispositiu del convidat, **signa els bundles amb ML-DSA-87**, i els puja al servidor. El servidor els guarda i els lliura quan el convidat els demana.

### Criptografia Utilitzada

| Algorisme | Estàndard | Ús | Nivell NIST |
|-----------|-----------|-----|-------------|
| **ML-KEM-1024** | FIPS 203 | Encapsulació de claus (rebre channel key) | Level 5 |
| **ML-DSA-87** | FIPS 204 | Signatures digitals (autenticar bundles) | Level 5 |
| **AES-GCM-256** | FIPS 197 | Xifratge simètric de missatges i wrapping de claus | — |

ML-KEM i ML-DSA no comparteixen format de clau, per tant cada dispositiu manté **dos keypairs separats**:

### Keypairs de Dispositiu

Cada dispositiu genera dos keypairs independents:

**1. ML-KEM-1024** — per rebre claus de canal (encapsulació):
- Clau pública: 1568 bytes → `devices.kem_public_key` al servidor
- Clau secreta: 3168 bytes → IndexedDB local, mai surt del dispositiu

**2. ML-DSA-87** — per signar bundles (autenticitat):
- Clau pública: 2592 bytes → `devices.dsa_public_key` al servidor
- Clau secreta: 4896 bytes → IndexedDB local, mai surt del dispositiu

```typescript
import { ml_kem1024 } from '@noble/post-quantum/ml-kem.js'
import { ml_dsa87 }   from '@noble/post-quantum/ml-dsa.js'

// Generació (una vegada per dispositiu)
const kem = ml_kem1024.keygen()
const dsa = ml_dsa87.keygen()

// Guardar secret keys localment
await indexedDB.put('keypairs', {
  deviceId,
  kemSecretKey: toBase64(kem.secretKey),
  dsaSecretKey: toBase64(dsa.secretKey),
})

// Pujar public keys al servidor
await fetch('/api/user/me/device/publickeys', {
  method: 'PUT',
  body: JSON.stringify({
    kemPublicKey: toBase64(kem.publicKey),   // per rebre claus
    dsaPublicKey: toBase64(dsa.publicKey),   // per verificar signatures
  })
})
```

**Payload que es signa** (tot concatenat en bytes):
```
SIGN_PAYLOAD = key_version_id || device_id || kem_ciphertext || encrypted_key
```
```typescript
const payload = concat(uuidToBytes(keyVersionId), uuidToBytes(deviceId), kemCiphertext, encryptedKey)
const signature = ml_dsa87.sign(my_dsa_secret_key, payload)
```

El servidor guarda la `dsa_public_key` de cada dispositiu i permet als clients descarregar-la per verificar signatures rebudes.

### Versioning de Claus

Les claus asimètriques també tenen versions. Cada versió té un conjunt de bundles (un per dispositiu amb accés).

**Esquema de taules:**

```sql
channel_key_versions
  id              UUID PRIMARY KEY
  channel_id      UUID NOT NULL REFERENCES channels(id)
  version         INTEGER NOT NULL
  created_at      TIMESTAMP NOT NULL
  created_by      UUID REFERENCES users(id)
  deprecated_at   TIMESTAMP
  UNIQUE (channel_id, version)

channel_key_device_bundles
  id                  UUID PRIMARY KEY
  key_version_id      UUID NOT NULL REFERENCES channel_key_versions(id)
  device_id           UUID NOT NULL REFERENCES devices(id)
  encrypted_key       TEXT NOT NULL    -- AES-GCM(shared_secret, channel_key)
  kem_ciphertext      TEXT NOT NULL    -- output ML-KEM.Encapsulate(device.public_key)
  signature           TEXT             -- ML-DSA-87.sign(signed_by_dsa_sk, payload)
  signed_by_device_id UUID             -- qui ha signat
  created_at          TIMESTAMP NOT NULL
  UNIQUE (key_version_id, device_id)

messages
  id              UUID PRIMARY KEY
  channel_id      UUID NOT NULL
  key_version_id  UUID REFERENCES channel_key_versions(id)
  ...
```

### Flux de Creació de Canal Asimètric

```
CLIENT (creador)                              SERVIDOR
  │                                             │
  │── Generar AES-256 channel_key (local)       │
  │── GET /api/channels/{id}/member-devices ───▶│
  │◀── [{ deviceId, publicKey }] ───────────────│
  │                                             │
  │ Per cada dispositiu (incl. els propis):     │
  │── ML-KEM.Encapsulate(device.publicKey)      │
  │     → (shared_secret, kem_ciphertext)       │
  │── AES-GCM.Encrypt(shared_secret, channel_key) → encrypted_key
  │── Sign(my_secret_key, keyVersionId + deviceId + encrypted_key + kem_ciphertext)
  │                                             │
  │ POST /api/channels/{id}/keys               ▶│
  │ [{ deviceId, encryptedKey, kemCiphertext,   │
  │    signature, signedByDeviceId,             │
  │    keyVersion: 1 }]                         │
  │                                             │── INSERT channel_key_versions (version=1)
  │                                             │── INSERT bundles per device
  │◀── { keyVersionId, version: 1 } ───────────│
  │                                             │
  │── IndexedDB.store(channelId, version=1, channel_key_xifrada_en_repos)
```

### Flux de Convit (Invitar Membre)

El client convidant ha de tenir la `channel_key` localment. Si no la té, no pot convidar ningú (limitació del model zero-knowledge).

```
CLIENT (convidant, té channel_key)            SERVIDOR
  │                                             │
  │── GET /api/users/{username}/devices ───────▶│
  │◀── [{ deviceId, publicKey, label }] ────────│
  │                                             │
  │ Per cada dispositiu del convidat:           │
  │── Validar publicKey (mida correcta ML-KEM-1024)
  │── ML-KEM.Encapsulate(device.publicKey)      │
  │── AES-GCM.Encrypt(shared_secret, channel_key) → encrypted_key
  │── Sign(my_secret_key, ...)                  │
  │                                             │
  │ POST /api/channels/{id}/keys ──────────────▶│
  │ [{ deviceId, encryptedKey, kemCiphertext,   │
  │    signature, signedByDeviceId }]           │
  │                                             │── Verificar que signed_by_device_id és membre del canal
  │                                             │── INSERT bundles
  │◀── { bundlesAdded: N } ─────────────────────│
```

### Flux d'Accés al Canal (Membre Convidat)

```
CLIENT (convidat)                             SERVIDOR
  │                                             │
  │ GET /api/channels/{id}/keys ───────────────▶│
  │                                             │── Buscar bundle per device_id del JWT
  │                                             │── Si no existeix → 404 ChannelKeyNotFound
  │◀── { keyVersionId, encryptedKey,            │
  │      kemCiphertext, signature,              │
  │      signedByDeviceId } ────────────────────│
  │                                             │
  │── GET /api/devices/{signedByDeviceId}/publickey (caché o nou request)
  │── Verificar signatura sobre el bundle
  │── Si signatura KO → Rebutjar. Mostrar avís: "Bundle no verificat"
  │── ML-KEM.Decapsulate(my_secret_key, kem_ciphertext) → shared_secret
  │── AES-GCM.Decrypt(shared_secret, encrypted_key) → channel_key
  │── IndexedDB.store(channelId, keyVersionId, channel_key_xifrada_en_repos)
  │── Desxifrar missatges amb channel_key
```

### Multi-Dispositiu: Distribució als Propis Dispositius

Quan un usuari obté la clau d'un canal asimètric (via convit o backup), pot distribuir-la als seus altres dispositius sense dependre de ningú extern:

```
CLIENT (dispositiu A, té channel_key)         SERVIDOR
  │                                             │
  │── GET /api/user/me/devices ───────────────▶│
  │◀── [{ deviceId, publicKey, label,           │
  │       isCurrent, hasPublicKey }] ───────────│
  │                                             │
  │ Per cada dispositiu propi (excl. dispositiu A):
  │── ML-KEM.Encapsulate(device.publicKey)      │
  │── AES-GCM.Encrypt(shared_secret, channel_key)
  │── Sign(my_secret_key, ...)                  │
  │                                             │
  │ POST /api/channels/{id}/keys ──────────────▶│
  │◀── { bundlesAdded: N } ─────────────────────│
```

Quan el dispositiu B obre el canal, trobarà el seu bundle i podrà desencriptar automàticament.

### Rotació de Clau Asimètrica (Revocació de Membre)

Quan cal revocar un membre (o dispositiu):

1. Admin genera nova `channel_key` (nova versió)
2. Distribueix la nova clau a tots els dispositius **excepte** el revocat — seguint el flux de convit
3. Missatges nous s'encripten amb la nova versió
4. L'expulsat no obté bundle per a la nova versió → no pot llegir missatges nous

Els missatges anteriors a la rotació que el revocat té en caché local no es poden eliminar retroactivament (limitació acceptada del model E2EE).

### Gestió de Dispositius

Cada usuari pot veure i gestionar els seus dispositius des de la UI:

**GET /api/user/me/devices** retorna:
```json
[
  {
    "deviceId": "uuid",
    "label": "MacBook Pro",
    "publicKey": "base64...",
    "hasPublicKey": true,
    "createdAt": "2026-01-01T00:00:00Z",
    "lastSeen": "2026-05-22T10:00:00Z",
    "isCurrent": true
  }
]
```

**Accions disponibles per dispositiu:**

| Acció | Descripció |
|-------|------------|
| Revocar | Elimina el dispositiu del servidor. Perd accés a futurs canals asimètrics |
| Canviar etiqueta | Reanomenar el dispositiu |
| Regenerar keypair | Genera nou keypair ML-KEM. Bundles antics queden obsolets. Cal redistribuir claus |
| Distribuir claus | Des del dispositiu actual, envia channel keys a un dispositiu propi seleccionat |

**La pantalla de gestió de dispositius ha de mostrar:**
- Llistat de dispositius amb etiqueta, data de creació, darrer accés
- Indicador "Dispositiu actual"
- Avís si un dispositiu no té clau pública registrada (no pot rebre claus asimètriques)
- Botó per revocar dispositius aliens
- Botó per distribuir les claus dels canals als dispositius que les necessiten

### Limitacions del Nivell 2

- ⚠️ Si perds la clau secreta sense backup ni altres dispositius → **pèrdua permanent** d'accés als missatges asimètrics
- ⚠️ La distribució de claus requereix que el convidant estigui en línia i tingui la clau localment
- ⚠️ No hi ha perfect forward secrecy per missatge (mateixa channel_key durant tota la versió)
- ✅ **Servidor zero-knowledge** — mai pot desxifrar res
- ✅ **Per dispositiu** — cada dispositiu té el seu keypair independent
- ✅ **Signatures** — el client verifica qui ha generat els bundles
- ✅ **Multi-dispositiu** — distribució de clau entre els propis dispositius
- ✅ **Quantum-resistant** — ML-KEM-1024 és NIST Level 5

---

## Gestió de Keypairs de Dispositiu

### Creació (Registre o Primer Login)

En el moment que un usuari s'autentica i el seu dispositiu no té keypair local:

1. Frontend genera keypair ML-KEM-1024
2. Guarda `secretKey` a IndexedDB (clau: `deviceId`)
3. Puja `publicKey` al servidor via `PUT /api/user/me/device/publickey`

Això succeeix automàticament en background, de forma transparent per a l'usuari.

### Pèrdua de Keypair (Neteja de Navegador, Canvi de Dispositiu)

Si l'usuari perd la clau secreta local (neteja de dades, nou navegador, etc.):

- **Canals simètrics (N1)**: el servidor torna a lliurar la clau de canal amb el nou keypair → recuperació automàtica
- **Canals asimètrics (N2)**: cal que un altre dispositiu propi o un membre que tingui la clau redistribueixi els bundles → recuperació manual

Per això és important tenir múltiples dispositius registrats o fer backup de la clau secreta.

---

## Àudio/Vídeo E2EE (LiveKit)

### Session Keys

LiveKit suporta E2EE nativa amb **session keys** independents del xat. La distribució de la session key es fa a través del canal de xat xifrat del canal de veu (si n'hi ha), o via handshake manual.

---

## Comparació de Nivells

| Característica | Nivell 0 | Nivell 1 (Simètric) | Nivell 2 (Asimètric) |
|---|---|---|---|
| Missatges xifrats | ❌ | ✅ | ✅ |
| Servidor pot llegir | ✅ | ✅ (master key) | ❌ |
| Recuperació de clau perduda | — | ✅ Automàtic (servidor) | ⚠️ Requereix backup o redistribució |
| Versioning de clau | ❌ | ✅ | ✅ |
| Rotació de clau | ❌ | ✅ | ✅ (revoca exmembre) |
| Multi-dispositiu | — | ✅ Automàtic | ✅ Manual (distribució) |
| Signatures de bundles | ❌ | ❌ | ✅ |
| Quantum-resistant | ❌ | Parcialment (transport) | ✅ ML-KEM-1024 |
| Complexitat client | Baixa | Baixa | Alta |
| Gestió de dispositius | — | Opcional | Necessari |
