# ChillGroup v2 — Missatges Directes (DM) 1:1 Asimètrics

## Objectiu

Definir el model objectiu de DM com una conversa privada 1:1 amb encriptació asimètrica (nivell 2) i suport de TTL (`message_ttl`), evitant el model provisional de DM amb `channel_id = nil`.

## Decisió Tècnica

Cada conversa directa es modela com un canal privat 1:1:

- `channel_type = text`
- `encryption_type = asymmetric`
- `is_private = true`
- exactament 2 usuaris membres
- `message_ttl` configurable (en segons) o `NULL` per missatges permanents

Això permet reutilitzar el pipeline existent de claus versionades, bundles per dispositiu i missatges per canal.

## Requisits Funcionals

1. Obre o crea conversa 1:1 per parella d'usuaris.
2. Només els 2 usuaris poden veure i enviar missatges.
3. `message_ttl` s'aplica als missatges nous de la conversa.
4. Encriptació obligatòria en nivell 2 (asimètric).
5. Multi-dispositiu: cada dispositiu de cada usuari ha de poder rebre bundle.
6. Rotació de clau suportada (manual o en canvis de dispositiu/revocació).

## Model de Dades Proposat

S'aprofiten taules existents amb una extensió petita de `channels`.

### Canvis a `channels`

Afegir metadades de DM:

```sql
ALTER TABLE channels
  ADD COLUMN scope VARCHAR(10) NOT NULL DEFAULT 'server'
    CHECK (scope IN ('server', 'dm'));

ALTER TABLE channels
  ALTER COLUMN server_id DROP NOT NULL;

ALTER TABLE channels
  ADD COLUMN dm_user_a_id UUID REFERENCES users(id),
  ADD COLUMN dm_user_b_id UUID REFERENCES users(id);
```

Regles d'integritat (aplicació + DB):

1. Si `scope = 'dm'`:
   1. `server_id IS NULL`
   2. `dm_user_a_id IS NOT NULL`
   3. `dm_user_b_id IS NOT NULL`
   4. `dm_user_a_id <> dm_user_b_id`
   5. `channel_type = 'text'`
   6. `encryption_type = 'asymmetric'`
   7. `is_private = true`
2. Unicitat de conversa 1:1 independent de l'ordre:

```sql
CREATE UNIQUE INDEX uniq_dm_pair
ON channels (
  LEAST(dm_user_a_id, dm_user_b_id),
  GREATEST(dm_user_a_id, dm_user_b_id)
)
WHERE scope = 'dm' AND deleted_at IS NULL;
```

Nota: si no existeix `deleted_at` a `channels`, substituir el `WHERE` per `WHERE scope = 'dm'`.

### Membres del canal

Es reutilitza `channel_members`:

- 2 files per DM (`user_a`, `user_b`)
- permisos resolts via `channel_members` igual que canals privats

### Claus i missatges

Sense canvis estructurals:

- `channel_key_versions` per versions de clau
- bundles per dispositiu com en qualsevol canal asimètric
- `messages.channel_id` continua sent la referència única

TTL:

- `channels.message_ttl` defineix expiració per defecte de la conversa
- `messages.expires_at = now() + message_ttl` en crear missatge

## API Proposada

Els endpoints de DM passen a operar sobre un `dmChannelId` real.

### 1. Obrir o crear DM

`POST /api/dm/channels/open`

Request:

```json
{
  "targetUserId": "uuid",
  "messageTTL": 86400
}
```

Resposta:

```json
{
  "success": true,
  "data": {
    "dmChannelId": "uuid",
    "peer": {
      "userId": "uuid",
      "username": "marcus"
    },
    "encryptionType": "asymmetric",
    "messageTTL": 86400,
    "keyVersionId": "uuid",
    "keyVersion": 1,
    "created": true
  }
}
```

Comportament:

1. Si ja existeix DM per la parella, retorna el mateix `dmChannelId` amb `created = false`.
2. Si no existeix:
   1. crea canal `scope=dm`
   2. crea membres (2 usuaris)
   3. crea `channel_key_versions` inicial
   4. el client creador distribueix bundles per dispositiu (els dos usuaris)

### 2. Llistar converses DM

`GET /api/dm/channels`

Resposta inclou:

- `dmChannelId`
- `peer` (usuari contrari)
- `lastMessageAt`
- `unreadCount`
- `messageTTL`

### 3. Missatges de DM

`GET /api/dm/channels/:dmChannelId/messages`

`POST /api/dm/channels/:dmChannelId/messages`

Mateix payload que canals normals (`encryptedPayload`, `iv`, `expiresAt` opcional).

Regla TTL:

1. si `expiresAt` ve informat, es valida que no superi política màxima;
2. si no ve informat i el canal té `message_ttl`, el backend el calcula automàticament.

### 4. Configurar TTL del DM

`PUT /api/dm/channels/:dmChannelId/settings`

Request:

```json
{
  "messageTTL": 3600
}
```

Només el creador del DM (o tots dos membres, segons política final) pot canviar TTL.

### 5. Claus de DM

Per simplificar implementació, es reutilitzen endpoints de claus de canal:

- `GET /api/channels/:channelId/keys`
- `POST /api/channels/:channelId/keys`

Amb validació de membre del DM.

## Seguretat i Privacitat

1. Servidor zero-knowledge: la clau de contingut no existeix en clar al servidor.
2. Signatura de bundles obligatòria (ML-DSA-87).
3. Distribució de claus per dispositiu, no per usuari.
4. Revocació de dispositiu implica redistribució de versions noves.
5. Els missatges expirats no es retornen en llistats i s'eliminen en neteja programada.

## Migració del Model Provisional

Model actual provisional:

- DM amb `channel_id = nil`
- sense canal real ni key version robusta

Migració proposada:

1. Marcar endpoints antics com `legacy`:
   1. `POST /api/direct-messages`
   2. `GET /api/direct-messages/list`
2. Crear nous endpoints `api/dm/channels/*`.
3. Nova UI de converses utilitzant `dmChannelId`.
4. Deixar compatibilitat temporal de lectura sobre legacy durant una versió.

## Pla d'Implementació (Fases)

1. Backend schema:
   1. migració de `channels` (`scope`, parella DM, nullable `server_id`)
   2. índex únic per parella
2. Backend routes:
   1. `POST /api/dm/channels/open`
   2. `GET /api/dm/channels`
   3. `GET/POST /api/dm/channels/:id/messages`
   4. `PUT /api/dm/channels/:id/settings`
3. Crypto flow:
   1. bootstrap key version en creació
   2. distribució bundles en obertura
4. Frontend:
   1. llista converses DM
   2. vista de conversa per `dmChannelId`
   3. modal de TTL DM
5. Tests:
   1. creació idempotent de DM
   2. accés prohibit a tercers
   3. expiració TTL efectiva
   4. recuperació de clau en dispositiu nou

## Criteris d'Acceptació

1. Dos usuaris obren DM i comparteixen missatges xifrats amb clau asimètrica.
2. Cap tercer usuari pot llegir ni llistar el DM.
3. `message_ttl` provoca `expires_at` i neteja real dels missatges.
4. Un segon dispositiu d'un membre pot rebre clau i desencriptar historial permès.
5. Endpoints legacy continuen funcionant temporalment sense regressió.
