# TODO 

1. La campana d'activar/desactivar notificacions, no hauria d'estar al bloc "channel-list-bottom-controls", potser hem de canviar una mica el menú i fer una opció de "configura usuari" que hi hagi: el canvi de password (compte i vault), pla de subscripció i activar o desctivar notificacions.

2. Si fem click a un missatges a la ℹ️ i es mostra, en cas que tingui un ttl el temps que falta per ser eliminta, si pots editar el missatge (ets admin o el creador), pots sumar temps al ttl o fins hi tot treure el ttl.

3. Els canals de veu comencen amb TAB i no amb FIX.

4. (Plantejar) Missatges destacats, els administradors poden posar pins a missatges. En el títol del canal hi haurà un pin que mostrarà els missatges amb pin.

Planteix:
Punt 4: Missatges destacats (pins) — com fer-ho
Backend
Nova taula pinned_messages:


CREATE TABLE pinned_messages (
  id UUID PRIMARY KEY,
  channel_id UUID NOT NULL REFERENCES channels(id),
  message_id UUID NOT NULL REFERENCES messages(id),
  pinned_by UUID NOT NULL REFERENCES users(id),
  pinned_at TIMESTAMPTZ NOT NULL,
  UNIQUE(channel_id, message_id)
);
Nous endpoints:

POST /api/channels/:id/pins — body { messageId } — requèreix permís MANAGE
DELETE /api/channels/:id/pins/:messageId — requèreix permís MANAGE
GET /api/channels/:id/pins — retorna llista de missatges pinats (amb contingut desxifrat pel client)
Esdeveniments Socket.IO:

message-pinned / message-unpinned → { channelId, messageId }
Frontend
ChannelHeader: afegir icona 📌 al títol. Click obre PinnedMessagesPanel (drawer lateral o popover).

PinnedMessagesPanel: llista els missatges pinats. Cada missatge mostra text (desxifrat), emissor i data de pin. Admins veuen botó de despin.

MessageList: al menú d'accions del missatge (hover), admins veuen botó 📌 per pinar/despinar.

Estat: pinnedMessages: Message[] a useAppState, refrescat en seleccionar canal i per socket events.

Consideració de xifrat
Els missatges estan xifrats E2E. El PinnedMessagesPanel necessita la clau del canal per desxifrar-los → ja disponible via decryptMessagesForChannel (el mateix flow que MessageList).

Complexitat estimada: mitjana-alta (nova taula BD, 3 endpoints, socket events, nou component UI, xifrat). Recomanació: atacar-ho en una sessió dedicada.