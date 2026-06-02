# TODO

## Context

La capa base de plans/tiers ja existeix (models, endpoints, límits hard i UI admin). Aquest TODO és per ampliar-la.

## P0 (prioritat alta)

* Docker mínim (només LiveKit + S3) per desenvolupar/provar localment.
* Afegir variable d'entorn `MAX_FILE_SIZE` per limitar la mida màxima per fitxer en upload.
* Estendre els plans actuals amb quotes de fitxers (S3), integrat amb el sistema de plans existent:
    * Free: 10 GB espai, 100 GB transferència mensual.
    * Pro: 50 GB espai, 500 GB transferència mensual.
    * Enterprise: 200 GB espai, transferència il·limitada.
* Persistir i calcular consum mensual de fitxers (espai i transferència) per usuari.
* Aplicar enforcement als endpoints d'adjunts (bloqueig quan se supera el límit).

### P0 desglossat per implementació

1. Docker mínim per desenvolupament local
    * Crear `docker-compose.minimal.yml` amb només `livekit`, `rustfs`, `rustfs-init` i `rustfs-cors-init`.
    * Reutilitzar `.env.compose` i `.env.compose.local` per no duplicar configuració.
    * Criteri de fet: aixecar stack mínima sense Postgres ni app i poder fer uploads a bucket i connexió LiveKit.

2. Límit global de mida per fitxer (`MAX_FILE_SIZE`)
    * Afegir `max_file_size` a `Config` a `server/src/config.rs` (amb default segur si no hi és).
    * Validar `req.size_bytes` a `init_attachment` de `server/src/routes/attachments.rs`.
    * Retornar error de negoci clar (codi + missatge) quan el fitxer supera el límit.
    * Criteri de fet: test que valida que un upload superior a `MAX_FILE_SIZE` retorna error i un inferior funciona.

3. Modelatge de quotes d'S3 al sistema de plans existent
    * Crear migració SQL nova a `server/migrations/` per afegir límits d'S3 a `plans`:
      * `max_storage_bytes`
      * `max_transfer_bytes_monthly`
    * Actualitzar model/lectura de plans a backend i API pública perquè exposi aquests nous camps.
    * Ajustar inicialització de plans per defecte a `server/src/db.rs` (free/pro/enterprise amb valors acordats).
    * Criteri de fet: `GET /api/plans` i `GET /api/user/me/limits` inclouen els nous límits.

4. Persistència del consum mensual d'S3
    * Crear taula nova (exemple: `user_storage_usage_monthly`) amb:
      * `user_id`, `year_month`, `stored_bytes`, `transfer_bytes`, timestamps.
    * Guardar/actualitzar consum en:
      * `complete_attachment` (increment de `stored_bytes`)
      * descàrregues (`download_attachment`/`download_attachment_proxy`) per `transfer_bytes`.
    * Definir política de decrement de `stored_bytes` quan s'esborren adjunts (si no existeix encara, deixar task marcada com a dependència).
    * Criteri de fet: consum visible i coherent després de pujada/baixada real.

5. Enforcement de quotes d'S3 a adjunts
    * A `init_attachment`, comprovar abans de crear multipart:
      * `stored_bytes + req.size_bytes <= max_storage_bytes`
      * quota mensual de transferència (aplicable en descàrrega).
    * A endpoints de descàrrega, bloquejar si `transfer_bytes_monthly` supera límit.
    * Mantenir `-1` com unlimited, coherent amb la resta de límits.
    * Criteri de fet: usuari free queda bloquejat quan supera límit; pro/enterprise mantenen comportament esperat.

6. Tests P0 (backend)
    * Ampliar tests de `server/src/routes/attachments.rs` amb casos:
      * fitxer > `MAX_FILE_SIZE`
      * límit espai assolit
      * límit transferència mensual assolit
      * `-1` unlimited
    * Afegir tests de capa DB per taula de consum mensual i càlcul acumulat.
    * Criteri de fet: suite de tests passa i cobreix paths d'error i d'èxit.

## P1 (prioritat mitjana)

* Estendre els plans amb quotes d'audio/video (LiveKit), integrat amb el sistema de plans existent:
    * Free: 10 hores de streaming mensual.
    * Pro: 50 hores de streaming mensual.
    * Enterprise: 200 hores de streaming mensual.
* Persistir i calcular consum mensual de streaming per usuari.
* Implementar notificacions de límit de pla (avisos al 80/90% i bloqueig al 100%).
* Crear pàgina d'autoservei de subscripció per a usuaris (veure pla actual + flux de canvi de pla).

## P2 (documentació i qualitat)

* Actualitzar documentació a `definitions/` i `docs-site/` amb els nous límits i comportament d'errors.
* Afegir tests d'integració per quotes d'S3 i LiveKit (free/pro/enterprise).